//! XVI packet builders. Every packet is a 65-byte feature report (family-e 2.2, family-c C2.2):
//! `[0]=0 [3]=target (2 mouse) [4]=data length [5]=category [6]=command [7..]=data`; profile-scoped
//! commands carry the profile id at `[7]` ("new protocol").

use crate::platform::{ModelSpec, PlanError};
use crate::profile::{
    ACTION_FORWARD, ACTION_BACKWARD, ACTION_LEFT, ACTION_MIDDLE, ACTION_OFF, ACTION_RIGHT, ACTION_SHORTCUT, Profile,
};
use crate::protocol::actions::{Group, info};
use crate::protocol::events::Event;

pub const REPORT_LEN: usize = 65;
const TARGET_MOUSE: u8 = 2;
/// Device profile the app writes to (R6 has 1..3, R5 Ultra 1..5).
const PROFILE: u8 = 1;
const ACK: u8 = 0xA1;
const MAX_DPI_STAGES: usize = 6;
/// Sleep range 15..=1800 s (family-e 3.2).
const MAX_SLEEP_SECS: u16 = 1800;
/// Wire polling codes for 125..8000 Hz (family-e 3.2).
const POLLING_CODES: [u8; 7] = [8, 4, 2, 1, 32, 64, 128];

fn raw(cat: u8, cmd: u8, len: u8, data: &[u8]) -> Vec<u8> {
    let mut b = vec![0u8; REPORT_LEN];
    (b[3], b[4], b[5], b[6]) = (TARGET_MOUSE, len, cat, cmd);
    b[7..7 + data.len()].copy_from_slice(data);
    b
}

/// Profile-scoped set: `[4]` = profile byte + `data`.
fn set(cat: u8, cmd: u8, data: &[u8]) -> Vec<u8> {
    let mut d = vec![PROFILE];
    d.extend_from_slice(data);
    raw(cat, cmd, d.len() as u8, &d)
}

pub fn set_active_profile() -> Vec<u8> {
    raw(0, 5, 1, &[PROFILE])
}

pub fn reset_profile() -> Vec<u8> {
    raw(0, 0x0D, 1, &[PROFILE])
}

pub fn polling(p: &Profile) -> Vec<Vec<u8>> {
    vec![set(1, 0, &[POLLING_CODES[usize::from(p.polling_index.min(6))]])]
}

/// DPI stages (cmd 1) + active stage (cmd 2). Stages are the enabled, non-zero slots in order.
pub fn dpi(spec: &ModelSpec, p: &Profile) -> Vec<Vec<u8>> {
    let slots: Vec<usize> = (0..p.dpi_units.len()).filter(|&i| p.stage_enabled[i] && p.dpi_units[i] > 0).take(MAX_DPI_STAGES).collect();
    let slots = if slots.is_empty() { vec![0] } else { slots };
    // inferred: the wire BE16 is the DPI itself (family-e 3.3); gated by `dpi.verified`
    let mut data = vec![slots.len() as u8];
    for &i in &slots {
        let dpi = (u32::from(p.dpi_units[i].clamp(1, spec.dpi.max_units)) * 50) as u16;
        data.extend_from_slice(&dpi.to_be_bytes());
        data.extend_from_slice(&dpi.to_be_bytes());
    }
    // inferred: active stage is 0-based and counts only the stages written (family-c C2.3 cmd 0x02)
    let active = slots.iter().position(|&i| i == usize::from(p.active_stage)).unwrap_or(0) as u8;
    vec![set(1, 1, &data), set(1, 2, &[active])]
}

pub fn options(spec: &ModelSpec, p: &Profile) -> Vec<Vec<u8>> {
    let secs = (u16::from(p.sleep_minutes) * 60).min(MAX_SLEEP_SECS);
    let mut out = vec![
        set(1, 4, &[u8::from(p.angle_snap)]),
        set(1, 0x0A, &[u8::from(p.ripple_control)]),
        set(1, 9, &[u8::from(p.motion_sync)]),
        set(1, 7, &secs.to_be_bytes()),
    ];
    // debounce is not sent: its command id collides with LOD (cmd 1/0x08, family-e 3.2 note)
    if !spec.lod_options.is_empty() {
        out.push(set(1, 8, &[spec.lod_options[usize::from(p.lift_off.min(1))]]));
    }
    out
}

/// Colors of the 6 DPI stages (family-e 3.4).
pub fn colors(p: &Profile) -> Vec<Vec<u8>> {
    let rgb: Vec<u8> = p.stage_colors.iter().take(MAX_DPI_STAGES).flatten().copied().collect();
    vec![set(2, 1, &rgb)]
}

/// Wire ButtonID for a profile button index (family-e 3.5); `None` = not on this mouse.
fn wire_button(index: usize) -> Option<u8> {
    Some(match index {
        0 => 1,
        1 => 2,
        2 => 3,
        6 => 4,
        7 => 5,
        _ => return None,
    })
}

/// `(functionID, data)` of a button action; `None` = no verified encoding (macros, double click, fire,
/// horizontal scroll, easy aim, LED loop), so no packet is sent for it.
fn function(b: &crate::profile::ButtonConfig) -> Option<(u8, Vec<u8>)> {
    let code = |c: u8| Some((1, vec![c]));
    match b.action_id {
        ACTION_OFF => Some((0, vec![])),
        ACTION_LEFT => code(1),
        ACTION_RIGHT => code(2),
        ACTION_MIDDLE => code(3),
        ACTION_BACKWARD => code(4),
        ACTION_FORWARD => code(5),
        ACTION_SHORTCUT => Some((4, vec![b.shortcut_mods, b.shortcut_key])),
        0x0B => code(16),
        0x0C => code(17),
        // inferred: DPI cycle = "cycle up" (param 6), DPI + / - = stage up / down (family-e 3.5)
        0x06 => Some((7, vec![6])),
        0x07 => Some((7, vec![1])),
        0x08 => Some((7, vec![2])),
        id => {
            let a = info(id)?;
            match a.group {
                // media / browser presets: wire[1..3] is the usage little-endian; the wire wants BE16
                Group::Media | Group::Browser => Some((5, vec![a.wire[2], a.wire[1]])),
                Group::Shortcut => Some((4, vec![a.wire[1], a.wire[2]])),
                _ => None,
            }
        }
    }
}

pub fn buttons(p: &Profile) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (i, b) in p.buttons.iter().enumerate() {
        let (Some(id), Some((func, data))) = (wire_button(i), function(b)) else { continue };
        let mut d = vec![id, 0, func, data.len() as u8];
        d.extend_from_slice(&data);
        out.push(set(3, 0, &d));
    }
    out
}

type GroupFn = fn(&ModelSpec, &Profile) -> Vec<Vec<u8>>;

/// Order: polling, DPI, options, colors, buttons. The DPI group (index 1) is the unverified one.
const GROUPS: [GroupFn; 5] =
    [|_, p| polling(p), dpi, options, |_, p| colors(p), |_, p| buttons(p)];
const DPI_GROUP: usize = 1;

pub fn plan(spec: &ModelSpec, prev: Option<&Profile>, new: &Profile, resync: bool, allow_unverified: bool)
-> Result<Vec<Vec<u8>>, PlanError> {
    let prev = if resync { None } else { prev };
    let mut out = Vec::new();
    if resync {
        out.push(reset_profile());
    }
    if prev.is_none() {
        out.push(set_active_profile());
    }
    for (i, build) in GROUPS.iter().enumerate() {
        let packets = build(spec, new);
        if prev.map(|p| build(spec, p)).as_ref() == Some(&packets) {
            continue;
        }
        if i == DPI_GROUP && !spec.dpi.verified && !allow_unverified {
            return Err(PlanError::UnverifiedEncoding);
        }
        out.extend(packets);
    }
    Ok(out)
}

/// Battery poll request (family-e 2.4, 3.1). The sender does SetFeature, waits 30 ms, GetFeature.
pub fn battery_request() -> Vec<u8> {
    raw(0, 0x83, 2, &[])
}

/// Reply to `request` is an ack when `[1] == 0xA1` and `[5]`/`[6]` echo the request (family-e 2.2).
pub fn is_ack(request: &[u8], reply: &[u8]) -> bool {
    reply.len() >= 7 && request.len() >= 7 && reply[1] == ACK && reply[5..7] == request[5..7]
}

/// Battery reply as the shared `Heartbeat` (`level` = percent), decodable with the model's `BatterySpec`.
/// The charge flag (`[7]`, 1 = charging) is mapped onto the C1 rule: 3 = charging, 1 = discharging.
// inferred: any flag other than 1 means "not charging"; full-charge value is not documented (family-e 3.1)
pub fn parse_battery(reply: &[u8]) -> Option<Event> {
    if !is_ack(&battery_request(), reply) || reply.len() < 9 {
        return None;
    }
    Some(Event::Heartbeat { state: if reply[7] == 1 { 3 } else { 1 }, level: reply[8] })
}
