//! Builders for the fixed-size commands 0x04, 0x05, 0x06, 0x08 and 0x0C (spec section 3).

use super::actions::wire_triplet;
use super::{CMD_BUTTONS, CMD_LIGHTING, CMD_POLLING, CMD_RESYNC, CMD_SENSOR, seal};
use crate::profile::{BUTTON_COUNT, DPI_STAGES, Profile};

const LIGHT_MODES_WITH_BRIGHTNESS: [u8; 2] = [1, 9];
/// Report interval in ms for each polling index (125/250/500/1000 Hz).
const POLLING_INTERVAL_MS: [u8; 4] = [8, 4, 2, 1];

/// Command 0x04: sensor + DPI (56 bytes).
pub fn build_sensor(p: &Profile) -> Vec<u8> {
    let mut b = vec![0u8; 56];
    b[..3].copy_from_slice(&[CMD_SENSOR, 0x38, 0x01]);
    b[3] = p.lift_off;
    b[4] = u8::from(p.ripple_control);
    b[5] = p.stage_enabled.iter().enumerate().fold(0, |m, (i, on)| m | (u8::from(*on) << i));
    b[6] = u8::from(p.angle_snap);
    b[7] = u8::from(p.motion_sync);
    for i in 0..DPI_STAGES {
        let encoded = p.dpi_units[i].saturating_sub(1);
        b[8 + i] = (encoded & 0xFF) as u8;
        b[16 + i] = (encoded >> 8) as u8;
        b[25 + 3 * i..28 + 3 * i].copy_from_slice(&p.stage_colors[i]);
    }
    b[24] = p.active_stage.saturating_add(1);
    b[49] = 0x01;
    seal(&mut b, 3, 49);
    b
}

/// Command 0x05: lighting + power + debounce (15 bytes), family A rules.
pub fn build_lighting(p: &Profile) -> Vec<u8> {
    build_lighting_with(p, Some(&LIGHT_MODES_WITH_BRIGHTNESS), true)
}

/// Command 0x05 with the per-family rules: brightness goes to `[5]` only for `brightness_modes`
/// (`None` = always, else `8`), and the speed is sent as `9 - speed` when `speed_inverted`.
pub fn build_lighting_with(p: &Profile, brightness_modes: Option<&[u8]>, speed_inverted: bool) -> Vec<u8> {
    let mut b = vec![0u8; 15];
    b[..3].copy_from_slice(&[CMD_LIGHTING, 0x0F, 0x01]);
    b[3] = p.light_mode << 4;
    let speed = if speed_inverted { 9u8.saturating_sub(p.light_speed) } else { p.light_speed & 0x0F };
    b[4] = (p.deep_sleep_minutes & 0xF0) | speed;
    let brightness = if brightness_modes.is_none_or(|m| m.contains(&p.light_mode)) { p.brightness & 0x0F } else { 8 };
    b[5] = (p.deep_sleep_minutes << 4) | brightness;
    b[6..9].copy_from_slice(&p.light_color);
    b[9] = p.sleep_minutes;
    b[10] = p.debounce;
    seal(&mut b, 3, 10);
    b
}

/// Command 0x06: polling rate (9 bytes) with inverted check byte.
pub fn build_polling(p: &Profile) -> Vec<u8> {
    let v = POLLING_INTERVAL_MS[usize::from(p.polling_index.min(3))];
    vec![CMD_POLLING, 0x09, 0x01, v, !v, 0, 0, 0, 0]
}

/// Command 0x08: button map (59 bytes), 18 buttons x (type, a, b).
pub fn build_buttons(p: &Profile) -> Vec<u8> {
    let mut b = vec![0u8; 59];
    b[..3].copy_from_slice(&[CMD_BUTTONS, 0x3B, 0x01]);
    for (i, button) in p.buttons.iter().take(BUTTON_COUNT).enumerate() {
        b[3 + 3 * i..6 + 3 * i].copy_from_slice(&wire_triplet(button, i));
    }
    seal(&mut b, 3, 56);
    b
}

/// Command 0x0C: full resync / restore. Risky (spec 3.6): only on explicit user confirmation.
pub fn build_resync() -> Vec<u8> {
    vec![CMD_RESYNC, 0x0A, 0x01, 0xFE, 0x01, 0xFE, 0, 0, 0, 0]
}
