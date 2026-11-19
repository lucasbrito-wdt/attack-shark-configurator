//! Profile model mirroring the spec's profile struct (section 6.2), plus sanitizing of untrusted data.

use serde::{Deserialize, Serialize};

pub const DPI_STAGES: usize = 8;
pub const BUTTON_COUNT: usize = 18;
/// Highest slider unit of any model (F1: 800 x 50 = 40000 DPI); each model clamps to its own range.
pub const MAX_DPI_UNITS: u16 = 800;
pub const DPI_STEP: u32 = 50;
pub const MAX_MACRO_EVENTS: usize = 256;
pub const MAX_NAME_LEN: usize = 48;
/// Buttons shown by the original UI (1,2,3,7,8 in 1-based numbering).
pub const VISIBLE_BUTTONS: [usize; 5] = [0, 1, 2, 6, 7];

pub type Rgb = [u8; 3];

pub const ACTION_OFF: u8 = 0x00;
pub const ACTION_LEFT: u8 = 0x01;
pub const ACTION_RIGHT: u8 = 0x02;
pub const ACTION_MIDDLE: u8 = 0x03;
pub const ACTION_FORWARD: u8 = 0x04;
pub const ACTION_BACKWARD: u8 = 0x05;
pub const ACTION_SHORTCUT: u8 = 0x10;
pub const ACTION_MACRO: u8 = 0x11;

pub const PLAY_COUNT: u8 = 1;
pub const PLAY_UNTIL_KEY: u8 = 2;
pub const PLAY_WHILE_HELD: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MacroEvent {
    /// HID keyboard usage, or a mouse code (see `protocol::macros`).
    pub key: u8,
    pub down: bool,
    /// Pause after this event, in milliseconds.
    pub delay_ms: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Macro {
    pub name: String,
    pub events: Vec<MacroEvent>,
    pub play_mode: u8,
    pub loop_count: u8,
}

impl Default for Macro {
    fn default() -> Self {
        Self { name: "Macro".into(), events: Vec::new(), play_mode: PLAY_COUNT, loop_count: 1 }
    }
}

impl Macro {
    fn sanitize(&mut self) {
        truncate_chars(&mut self.name, MAX_NAME_LEN);
        self.events.truncate(MAX_MACRO_EVENTS);
        self.play_mode = self.play_mode.clamp(PLAY_COUNT, PLAY_WHILE_HELD);
        self.loop_count = self.loop_count.max(1);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ButtonConfig {
    pub action_id: u8,
    pub shortcut_key: u8,
    /// bit0 Ctrl, bit1 Shift, bit2 Alt, bit3 Win/Cmd.
    pub shortcut_mods: u8,
    pub macro_binding: Option<Macro>,
}

impl Default for ButtonConfig {
    fn default() -> Self {
        Self { action_id: ACTION_OFF, shortcut_key: 0, shortcut_mods: 0, macro_binding: None }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    /// 0 = 1 mm, 1 = 2 mm.
    pub lift_off: u8,
    pub ripple_control: bool,
    pub angle_snap: bool,
    pub motion_sync: bool,
    /// Vendor slider units (DPI = 50*u up to the model threshold, then 100 per unit); 0 = unused slot.
    pub dpi_units: [u16; DPI_STAGES],
    pub stage_enabled: [bool; DPI_STAGES],
    pub stage_colors: [Rgb; DPI_STAGES],
    /// 0-based active stage.
    pub active_stage: u8,
    /// 0..=6 = 125/250/500/1000/2000/4000/8000 Hz.
    pub polling_index: u8,
    pub debounce: u8,
    pub sleep_minutes: u8,
    pub deep_sleep_minutes: u8,
    pub light_mode: u8,
    pub brightness: u8,
    pub light_speed: u8,
    pub light_color: Rgb,
    pub buttons: Vec<ButtonConfig>,
}

impl Default for Profile {
    fn default() -> Self {
        let mut buttons = vec![ButtonConfig::default(); BUTTON_COUNT];
        for (idx, action) in [(0, ACTION_LEFT), (1, ACTION_RIGHT), (2, ACTION_MIDDLE), (6, ACTION_BACKWARD), (7, ACTION_FORWARD)] {
            // inferred: default button actions are L, R, M, Back, Fwd (spec unknown #13), verify with USB capture
            buttons[idx].action_id = action;
        }
        Self {
            name: "Default".into(),
            lift_off: 0,
            ripple_control: false,
            angle_snap: false,
            motion_sync: false,
            dpi_units: [16, 32, 48, 64, 100, 520, 0, 0],
            stage_enabled: [true, true, true, true, true, true, false, false],
            stage_colors: [
                [0xFF, 0, 0],
                [0, 0xFF, 0],
                [0, 0, 0xFF],
                [0xFF, 0xFF, 0],
                [0, 0xFF, 0xFF],
                [0xFF, 0, 0xFF],
                [0xFF, 0x40, 0],
                [0xFF, 0xFF, 0xFF],
            ],
            active_stage: 1,
            polling_index: 3,
            debounce: 4,
            sleep_minutes: 1,
            deep_sleep_minutes: 10,
            light_mode: 0,
            brightness: 8,
            light_speed: 6,
            light_color: [0, 0, 0xFF],
            buttons,
        }
    }
}

fn truncate_chars(s: &mut String, max: usize) {
    if let Some((idx, _)) = s.char_indices().nth(max) {
        s.truncate(idx);
    }
}

impl Profile {
    pub fn named(name: &str) -> Self {
        Self { name: name.to_string(), ..Self::default() }
    }

    /// Force every field into its valid range. Import files are untrusted.
    pub fn sanitize(&mut self) {
        truncate_chars(&mut self.name, MAX_NAME_LEN);
        if self.name.trim().is_empty() {
            self.name = "Profile".into();
        }
        self.lift_off = self.lift_off.min(1);
        for units in &mut self.dpi_units {
            *units = (*units).min(MAX_DPI_UNITS);
        }
        self.active_stage = self.active_stage.min(DPI_STAGES as u8 - 1);
        self.polling_index = self.polling_index.min(6);
        self.debounce = self.debounce.clamp(2, 25);
        self.sleep_minutes = self.sleep_minutes.clamp(1, 60);
        self.deep_sleep_minutes = self.deep_sleep_minutes.clamp(1, 60);
        self.light_mode = self.light_mode.min(11);
        self.brightness = self.brightness.clamp(1, 8);
        self.light_speed = self.light_speed.clamp(4, 8);
        self.buttons.resize_with(BUTTON_COUNT, ButtonConfig::default);
        for button in &mut self.buttons {
            if let Some(m) = &mut button.macro_binding {
                m.sanitize();
            }
        }
    }
}

/// Macro library entry sanitizing for imported files.
pub fn sanitize_macro(m: &mut Macro) {
    m.sanitize();
}
