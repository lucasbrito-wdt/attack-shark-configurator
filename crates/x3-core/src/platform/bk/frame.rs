//! 4K transport framing (family-b 3.4) and the read-only requests: 0xA0 read-back, 0x0B / BLE A1
//! firmware version (family-b 3.6, family-c C1.2), plus read-back decoding for the probe.

use super::codec::table_code;
use crate::model::dpi_from_units;
use crate::protocol::checksum16;

pub const REPORT_4K: u8 = 0x04;
const FRAME_PAYLOAD: usize = 59;
pub const CMD_READ: u8 = 0xA0;
pub const CMD_VERSION: u8 = 0x0B;
pub const CMD_BULK: u8 = 0x0A;

/// Frame `i` = `04 <n+5> <i>` + `pkt` chunk of n <= 59 bytes + BE sum of the frame so far,
/// zero-padded to `pad_to` (0x41 on the 4K receiver, 0x40 wired).
pub fn frame_4k(pkt: &[u8], pad_to: usize) -> Vec<Vec<u8>> {
    pkt.chunks(FRAME_PAYLOAD)
        .enumerate()
        .map(|(i, chunk)| {
            let mut f = vec![REPORT_4K, (chunk.len() + 5) as u8, i as u8];
            f.extend_from_slice(chunk);
            f.extend_from_slice(&checksum16(&f).to_be_bytes());
            if f.len() < pad_to {
                f.resize(pad_to, 0);
            }
            f
        })
        .collect()
}

/// SetFeature `A0 <id> <len> 00 <flag> 00 00 00`. Asks the device to stage `id` for GetFeature.
// inferred: flag = profile 1 (C1 names [4] "profile"; family B's value was not resolved)
pub fn read_request(id: u8, len: u8) -> [u8; 8] {
    [CMD_READ, id, len, 0, 0x01, 0, 0, 0]
}

/// GetFeature 0xA0 reply: ready when `resp[1] == 1`.
pub fn read_ready(resp: &[u8]) -> bool {
    resp.get(1) == Some(&1)
}

/// GetFeature 0x0B: `ver = resp[2] << 8 | resp[3]`.
pub fn parse_version(resp: &[u8]) -> Option<u16> {
    match resp {
        [CMD_VERSION, _, hi, lo, ..] => Some(u16::from_be_bytes([*hi, *lo])),
        _ => None,
    }
}

/// C1: version bytes `[4..7]` all 128 = X3, all 129 = X6 (both report model code 0x10).
pub fn version_model_hint(resp: &[u8]) -> Option<&'static str> {
    match resp.get(4..8)? {
        [128, 128, 128, 128] => Some("X3"),
        [129, 129, 129, 129] => Some("X6"),
        _ => None,
    }
}

/// Version `0x10xx` answers per-command reads (0x06, 0x05, 0x04, 0x08); others answer a 0x80-byte bulk 0x0A.
pub fn split_read(version: u16) -> bool {
    version & 0xFF00 == 0x1000
}

/// BLE FEE4 reply `A1 hi lo ..`.
pub fn ble_version(data: &[u8]) -> Option<u16> {
    match data {
        [0xA1, hi, lo, ..] => Some(u16::from_be_bytes([*hi, *lo])),
        _ => None,
    }
}

/// One stage of a 0x04 read-back under both conflicting interpretations (family-b unknown #1).
#[derive(Debug, PartialEq, Eq)]
pub struct StageDecode {
    pub stage: usize,
    pub enabled: bool,
    /// Family A: `enc = u - 1`, DPI = 50 * u.
    pub as_a: u32,
    /// Variant T (R1 page): every slider value whose code, half-step flag and `[6]` bit match.
    pub as_t: Vec<u32>,
}

/// Decode the DPI stages of a 56-byte 0x04 read-back.
pub fn decode_sensor(resp: &[u8]) -> Option<Vec<StageDecode>> {
    if resp.len() < 25 || resp[0] != 0x04 {
        return None;
    }
    let stages = (0..8)
        .map(|i| {
            let (lo, hi) = (resp[8 + i], resp[16 + i]);
            let high = resp[6] & (1 << i) != 0;
            let as_t = (1..=350u16)
                .filter(|&u| table_code(u) == lo && u8::from((201..=220).contains(&u) || u >= 301) == hi && (u > 220) == high)
                .map(|u| dpi_from_units(Some(200), u))
                .collect();
            StageDecode { stage: i, enabled: resp[5] & (1 << i) != 0, as_a: (u32::from(u16::from_le_bytes([lo, hi])) + 1) * 50, as_t }
        })
        .collect();
    Some(stages)
}
