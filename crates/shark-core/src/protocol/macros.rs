//! Command 0x09 macro packet and the 3 x 64-byte chunking used on wired and dongle links.
//!
//! The event timing encoding constants were not decoded (spec unknown #7); everything below
//! that is marked inferred must be verified with a USB capture of a known macro.

use super::{CMD_MACRO, seal};
use crate::profile::{Macro, MacroEvent};

pub const MACRO_PACKET_LEN: usize = 131;
pub const CHUNK_LEN: usize = 64;
const CHUNK_PAYLOAD: usize = 60;
const MAX_PAIRS: usize = 50;
const FIRST_PAIR: usize = 29;
const FILLER_KEY: u8 = 0x03;
const FILLER_MS: u16 = 200;
const MAX_FILLERS: u16 = 10;
const MAX_SINGLE_DELAY_MS: u16 = 1270;
const KEY_UP_FLAG: u8 = 0x80;

fn timing_byte(delay_ms: u16, down: bool) -> u8 {
    // inferred: delay in 10 ms units (1270 ms max = 127 units) in bits 6..0, key-up flag in bit 7
    let units = ((delay_ms + 5) / 10).clamp(1, 127) as u8;
    if down { units } else { units | KEY_UP_FLAG }
}

/// Encode events as `(timing|flag, key)` pairs, splitting long delays into filler events.
pub fn encode_events(events: &[MacroEvent]) -> Vec<(u8, u8)> {
    let mut pairs = Vec::new();
    for ev in events {
        // inferred: delays above 1270 ms become up to 10 filler events of 200 ms, then the remainder
        let fillers = if ev.delay_ms > MAX_SINGLE_DELAY_MS { (ev.delay_ms / FILLER_MS).min(MAX_FILLERS) } else { 0 };
        for _ in 0..fillers {
            pairs.push((timing_byte(FILLER_MS, ev.down), FILLER_KEY));
        }
        let remainder = (ev.delay_ms - fillers * FILLER_MS).min(MAX_SINGLE_DELAY_MS);
        pairs.push((timing_byte(remainder, ev.down), ev.key));
    }
    pairs.truncate(MAX_PAIRS);
    pairs
}

/// Full 131-byte macro packet for `button_index` (0-based; sent as 1-based).
pub fn build_macro(button_index: usize, m: &Macro) -> Vec<u8> {
    let mut b = vec![0u8; MACRO_PACKET_LEN];
    b[..2].copy_from_slice(&[CMD_MACRO, 0x83]);
    b[2] = (button_index + 1) as u8;
    b[3] = m.play_mode;
    b[4..7].fill(m.loop_count);
    b[7] = 0x01;
    let pairs = encode_events(&m.events);
    // inferred: byte [28] holds the number of event pairs
    b[28] = pairs.len() as u8;
    for (i, (timing, key)) in pairs.iter().enumerate() {
        b[FIRST_PAIR + 2 * i] = *timing;
        b[FIRST_PAIR + 2 * i + 1] = *key;
    }
    seal(&mut b, 3, 128);
    b
}

/// Split a 131-byte macro packet into 3 chunks of 64 bytes (spec 2.3).
/// On the dongle the third chunk carries `[1] = 0x0C`.
pub fn chunk_macro(pkt: &[u8], dongle: bool) -> Vec<Vec<u8>> {
    if pkt.len() != MACRO_PACKET_LEN {
        return Vec::new();
    }
    let src = &pkt[3..];
    (0..3)
        .map(|idx| {
            let mut c = vec![0u8; CHUNK_LEN];
            c[..4].copy_from_slice(&[CMD_MACRO, CHUNK_LEN as u8, pkt[2], idx as u8]);
            let from = (CHUNK_PAYLOAD * idx).min(src.len());
            let to = (from + CHUNK_PAYLOAD).min(src.len());
            c[4..4 + to - from].copy_from_slice(&src[from..to]);
            if dongle && idx == 2 {
                c[1] = 0x0C;
            }
            c
        })
        .collect()
}
