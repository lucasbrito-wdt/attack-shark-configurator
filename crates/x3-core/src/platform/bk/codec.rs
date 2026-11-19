//! BK packet builders that differ per model: 0x04 variants A/T/H/direct, 0x05 per family,
//! 0x06 polling encodings and the C1 0x08 button types. Family A builders live in `protocol::packets`.

use super::registry::{BkDpi, BkFamily, BkWire, PollingEncoding, TableStyle};
use crate::platform::ModelSpec;
use crate::profile::{ACTION_MACRO, ACTION_SHORTCUT, BUTTON_COUNT, ButtonConfig, DPI_STAGES, Profile};
use crate::protocol::actions::{Group, info, wire_triplet};
use crate::protocol::packets::{build_buttons, build_lighting_with, build_sensor as build_sensor_a};
use crate::protocol::{CMD_BUTTONS, CMD_POLLING, CMD_SENSOR, seal};

/// Sensor codes for `u = 301..=340` (R1/V3 tables, family-b 3.1). `u = 301 -> 235` is the vendor's value.
const T_301_340: [u8; 40] = [
    235, 118, 118, 119, 119, 121, 121, 122, 122, 123, 123, 124, 124, 125, 125, 127, 127, 128, 128, 129, 129, 130, 130, 131,
    131, 132, 132, 134, 134, 135, 135, 136, 136, 137, 137, 138, 138, 139, 139, 141,
];
/// Sensor codes for `u = 341..=350` (V3 table).
const T_341_350: [u8; 10] = [189, 190, 191, 191, 192, 194, 194, 195, 196, 199];
const TABLE_MAX_UNITS: u16 = 350;

/// Variant T sensor code for slider units `u` (0 for an unused stage).
pub fn table_code(u: u16) -> u8 {
    let ceil = |a: u32, b: u32| a.div_ceil(b);
    let u = u.min(TABLE_MAX_UNITS);
    match u {
        0 => 0,
        6 => 6,
        1..=200 => (ceil(100 * u32::from(u), 85) - 1) as u8,
        201..=300 => (ceil((u32::from(u) - 100) * 100, 85) - 1) as u8,
        301..=340 => T_301_340[usize::from(u - 301)],
        _ => T_341_350[usize::from(u - 341)],
    }
}

fn header(len: u8) -> Vec<u8> {
    let mut b = vec![0u8; usize::from(len)];
    b[..3].copy_from_slice(&[CMD_SENSOR, len, 0x01]);
    b
}

fn mask(p: &Profile) -> u8 {
    p.stage_enabled.iter().enumerate().fold(0, |m, (i, on)| m | (u8::from(*on) << i))
}

fn finish(b: &mut [u8], p: &Profile, colors: bool, trailer: u8) {
    b[5] = mask(p);
    b[24] = p.active_stage.saturating_add(1);
    if colors {
        for i in 0..DPI_STAGES {
            b[25 + 3 * i..28 + 3 * i].copy_from_slice(&p.stage_colors[i]);
        }
        b[49] = trailer;
    }
    seal(b, 3, 49);
}

fn nibbles(hi: bool, lo: bool) -> u8 {
    (u8::from(hi) << 4) | u8::from(lo)
}

/// Command 0x04 for `spec`, with every stage clamped to the model's slider range.
pub fn build_sensor(spec: &ModelSpec, wire: &BkWire, p: &Profile) -> Vec<u8> {
    let mut p = p.clone();
    for u in &mut p.dpi_units {
        *u = (*u).min(spec.dpi.max_units);
    }
    let units = p.dpi_units;
    match wire.dpi {
        BkDpi::A { trailer } => {
            let mut b = build_sensor_a(&p);
            b[49] = trailer;
            seal(&mut b, 3, 49);
            b
        }
        BkDpi::Table(style) => {
            let mut b = header(0x38);
            (b[3], b[4]) = match style {
                TableStyle::R1 => (u8::from(p.angle_snap), u8::from(p.ripple_control)),
                TableStyle::V3 => (u8::from(p.motion_sync), u8::from(p.angle_snap)),
            };
            for (i, &u) in units.iter().enumerate() {
                let (b6, b7) = match style {
                    TableStyle::R1 => (u > 220, u > 220),
                    TableStyle::V3 => ((221..=340).contains(&u), u >= 341),
                };
                b[6] |= u8::from(b6) << i;
                b[7] |= u8::from(b7) << i;
                b[8 + i] = table_code(u);
                let half = (201..=220).contains(&u) || u >= 301;
                b[16 + i] = u8::from(half && !(style == TableStyle::V3 && u >= 341));
            }
            finish(&mut b, &p, true, 0x02);
            b
        }
        BkDpi::High { colors } => {
            let t = spec.dpi.threshold.unwrap_or(spec.dpi.max_units);
            let mut b = header(0x38);
            b[3] = (p.lift_off << 4) | u8::from(p.ripple_control);
            b[4] = nibbles(p.angle_snap, p.motion_sync);
            for (i, &u) in units.iter().enumerate() {
                b[6] |= u8::from(u > t) << i;
                // above T: DPI/100 - 1 with DPI = (u - T/2) * 100
                let enc = if u > t { u - t / 2 - 1 } else { u.saturating_sub(1) };
                b[8 + i] = (enc & 0xFF) as u8;
                b[16 + i] = (enc >> 8) as u8;
            }
            finish(&mut b, &p, colors, 0x02);
            b
        }
        BkDpi::Direct { paw3395 } => {
            let mut b = header(0x38);
            (b[3], b[4]) = if paw3395 {
                ((p.lift_off << 4) | u8::from(p.ripple_control), nibbles(p.angle_snap, p.motion_sync))
            } else {
                (u8::from(p.angle_snap), u8::from(p.ripple_control))
            };
            for (i, &u) in units.iter().enumerate() {
                let raw = (u32::from(u) * 50).min(0xFFFF) as u16;
                b[8 + i] = (raw & 0xFF) as u8;
                b[16 + i] = (raw >> 8) as u8;
            }
            // inferred: indicator type [49] (C1: 1/2/3, per-model value not listed); 1 as in family A
            finish(&mut b, &p, true, 0x01);
            b
        }
    }
}

/// Lighting modes whose 0x05 packet carries the brightness (`None` = every mode); the others send 8.
pub fn brightness_modes(family: BkFamily) -> Option<&'static [u8]> {
    match family {
        BkFamily::A => Some(&[1, 9]),
        BkFamily::B => Some(&[1, 5]),
        BkFamily::C1 => None,
    }
}

/// Command 0x05 with the family's brightness modes and speed direction.
pub fn build_lighting(wire: &BkWire, p: &Profile) -> Vec<u8> {
    build_lighting_with(p, brightness_modes(wire.family), wire.family != BkFamily::C1)
}

/// Polling value byte for index 0..=6 (125..8000 Hz).
pub fn polling_value(encoding: PollingEncoding, index: u8) -> u8 {
    let idx = usize::from(index.min(6));
    match encoding {
        PollingEncoding::Interval => [8, 4, 2, 1][idx.min(3)],
        // shortcut: one profile rate drives both nibbles (ls capped at 1000 Hz), split them if the UI ever needs two
        PollingEncoding::DualNibble => (((idx as u8) + 1) << 4) | (idx.min(3) as u8 + 1),
        PollingEncoding::Interval250us => [0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0x40][idx],
    }
}

/// Command 0x06: `06 09 01 v ~v 00 00 00 00`.
pub fn build_polling(wire: &BkWire, p: &Profile) -> Vec<u8> {
    let v = polling_value(wire.polling, p.polling_index);
    vec![CMD_POLLING, 0x09, 0x01, v, !v, 0, 0, 0, 0]
}

/// C1 button type for an internal action id (PROTOCOL.md 3.7 -> family-c C1.5); `None` = no equivalent.
fn c1_type(id: u8) -> Option<u8> {
    Some(match id {
        0x00 => 1,
        0x01 => 2,
        0x02 => 3,
        0x03 => 4,
        0x04 => 6,
        0x05 => 5,
        0x06 => 13,
        0x07 => 14,
        0x08 => 15,
        0x09 => 7,
        0x0B => 9,
        0x0C => 10,
        0x0D => 11,
        0x0E => 12,
        0x16 => 16,
        0x17 => 41,
        0x40 => 21,
        0x41 => 24,
        0x42 => 25,
        0x43 => 22,
        0x44 => 23,
        0x45 => 27,
        0x46 => 28,
        0x47 => 26,
        0x50 => 37,
        0x51 => 31,
        0x52 => 32,
        0x53 => 33,
        0x54 => 34,
        0x55 => 36,
        0x56 => 38,
        0x57 => 30,
        0x58 => 29,
        0x59 => 35,
        _ => return None,
    })
}

const C1_KEYBOARD: u8 = 17;
const C1_MACRO: u8 = 18;

/// C1 `{type, code1, code2}` for button `index`.
pub fn c1_triplet(p: &Profile, index: usize) -> [u8; 3] {
    let b = &p.buttons[index];
    match b.action_id {
        ACTION_SHORTCUT => [C1_KEYBOARD, b.shortcut_mods, b.shortcut_key],
        ACTION_MACRO => [C1_MACRO, 0, (index + 1) as u8],
        // shortcut presets are keyboard combos: reuse their modifier and key bytes
        id => match (c1_type(id), info(id)) {
            (Some(t), _) => [t, 0, 0],
            (None, Some(a)) if a.group == Group::Shortcut => [C1_KEYBOARD, a.wire[1], a.wire[2]],
            _ => [1, 0, 0],
        },
    }
}

/// Command 0x08 (59 bytes): legacy triplets for families A/B, C1 types for C1.
pub fn build_button_map(spec: &ModelSpec, wire: &BkWire, p: &Profile) -> Vec<u8> {
    if wire.family != BkFamily::C1 {
        let mut b = build_buttons(p);
        // slots the app does not expose (DPI keys, wheel) keep the factory action: the profile holds
        // action 0 there, which encodes as 0x01 (disabled) and turned off the X6 wheel and DPI keys.
        // Without a factory table they stay 00 00 00, which kept the X6 wheel working.
        for i in (0..BUTTON_COUNT).filter(|i| !spec.buttons.iter().any(|x| usize::from(x.id) == *i)) {
            let factory = wire.factory_buttons.map(|t| ButtonConfig { action_id: t[i], ..ButtonConfig::default() });
            b[3 + 3 * i..6 + 3 * i].copy_from_slice(&factory.map_or([0; 3], |f| wire_triplet(&f, i)));
        }
        seal(&mut b, 3, 56);
        return b;
    }
    let mut b = vec![0u8; 59];
    b[..3].copy_from_slice(&[CMD_BUTTONS, 0x3B, 0x01]);
    for i in 0..BUTTON_COUNT.min(p.buttons.len()) {
        b[3 + 3 * i..6 + 3 * i].copy_from_slice(&c1_triplet(p, i));
    }
    seal(&mut b, 3, 56);
    b
}
