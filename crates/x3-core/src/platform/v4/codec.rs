//! V4 packet builders (family-c.md C3.2 / C3.3). Every packet is 64 bytes: `[0]` command, `[1..7)` args,
//! `[7]` checksum `0xFF - sum(b[0..7])`, `[8..]` data. The checksum does not cover the data.

use super::registry::{V4Class, V4Model};
use crate::platform::spec::ModelSpec;
use crate::profile::Profile;
use crate::model::dpi_from_units;

pub const PACKET_LEN: usize = 64;
pub const CMD_GET_ID: u8 = 0x8F;
const CMD_DEBOUNCE: u8 = 0x04;
const CMD_POLLING: u8 = 0x08;
const CMD_DPI: u8 = 0x10;
const CMD_LINE_REPAIR: u8 = 0x12;
const CMD_RIPPLE_REPAIR: u8 = 0x13;
const CMD_MOTION_SYNC: u8 = 0x14;
// Pan1080 reuses 0x04 for "quick polling" (`[1]` profile, `[2]` code) and 0x54 for DPI.
const CMD_PAN_POLLING: u8 = 0x04;
const CMD_PAN_DPI: u8 = 0x54;

/// Report-rate codes for 125, 250, 500, 1000, 2000, 4000, 8000 Hz (C3.3 `_encodeReportRate`).
const RATE_CODES: [u8; 7] = [0x08, 0x04, 0x02, 0x01, 0x84, 0x82, 0x81];

pub fn seal(mut b: Vec<u8>) -> Vec<u8> {
    b.resize(PACKET_LEN, 0);
    b[7] = 0xFF - b[..7].iter().fold(0u8, |s, v| s.wrapping_add(*v));
    b
}

fn command(args: &[u8]) -> Vec<u8> {
    seal(args.to_vec())
}

/// Version/id request. Reply: `r[0] == 0x8F`, id = LE32 at `r[1..5]`.
pub fn build_get_id() -> Vec<u8> {
    command(&[CMD_GET_ID])
}

pub fn parse_device_id(reply: &[u8]) -> Option<u32> {
    match reply {
        [CMD_GET_ID, a, b, c, d, ..] => Some(u32::from_le_bytes([*a, *b, *c, *d])),
        _ => None,
    }
}

pub fn build_polling(m: &V4Model, p: &Profile) -> Vec<u8> {
    let last = m.spec.polling_rates.len() - 1;
    let code = RATE_CODES[usize::from(p.polling_index).min(last)];
    match m.class {
        V4Class::Ch585 => command(&[CMD_POLLING, code]),
        // profile 0; quick command only knows 1000/500/250/125, all within the Pan1080 rate list
        V4Class::Pan1080 => command(&[CMD_PAN_POLLING, 0, code]),
    }
}

/// DPI params (CH585 0x10 / Pan1080 0x54, same layout): `[1]` profile, `[2]` active stage, `[3]` count,
/// X LE16 at `8+2i`, Y LE16 at `24+2i`, RGB at `40+3i`. Enabled stages are packed, at most `spec.dpi.stages`.
pub fn build_dpi(m: &V4Model, p: &Profile) -> Vec<u8> {
    let spec: &ModelSpec = &m.spec;
    let stages: Vec<usize> = (0..p.dpi_units.len()).filter(|&i| p.stage_enabled[i] && p.dpi_units[i] > 0).take(usize::from(spec.dpi.stages)).collect();
    let active = stages.iter().position(|&i| i == usize::from(p.active_stage)).unwrap_or(0);
    let cmd = if m.class == V4Class::Ch585 { CMD_DPI } else { CMD_PAN_DPI };
    let mut b = vec![0u8; PACKET_LEN];
    (b[0], b[1], b[2], b[3]) = (cmd, 0, active as u8, stages.len() as u8);
    for (n, &i) in stages.iter().enumerate() {
        // Y is sent equal to X: the profile has a single DPI per stage
        let dpi = dpi_from_units(None, p.dpi_units[i]).clamp(50, spec.dpi_max()) as u16;
        b[8 + 2 * n..10 + 2 * n].copy_from_slice(&dpi.to_le_bytes());
        b[24 + 2 * n..26 + 2 * n].copy_from_slice(&dpi.to_le_bytes());
        b[40 + 3 * n..43 + 3 * n].copy_from_slice(&p.stage_colors[i]);
    }
    seal(b)
}

/// Packets in send order. CH585 has one command per setting; Pan1080 keeps debounce, angle snap and the like
/// inside option param 0, which needs a read-modify-write, so only polling and DPI are produced there.
pub fn build_all(m: &V4Model, p: &Profile) -> Vec<Vec<u8>> {
    let mut out = vec![build_dpi(m, p), build_polling(m, p)];
    if m.class == V4Class::Ch585 {
        out.push(command(&[CMD_DEBOUNCE, p.debounce]));
        out.push(command(&[CMD_LINE_REPAIR, u8::from(p.angle_snap)]));
        out.push(command(&[CMD_RIPPLE_REPAIR, u8::from(p.ripple_control)]));
        out.push(command(&[CMD_MOTION_SYNC, u8::from(p.motion_sync)]));
    }
    out
}

pub fn is_dpi(pkt: &[u8]) -> bool {
    matches!(pkt.first(), Some(&(CMD_DPI | CMD_PAN_DPI)))
}
