//! Button action table (spec 3.4 and 3.7) and the `(type, a, b)` wire encoding.
//!
//! The lookup table at 0x41C4A8 was only partially decoded (spec unknown #6), so the wire
//! triplets of every action except keyboard shortcuts are inferred.

use crate::profile::{ACTION_MACRO, ACTION_SHORTCUT, ButtonConfig};

// inferred: type byte per action family, verify with USB capture
const T_MOUSE: u8 = 0x01;
const T_KEYBOARD: u8 = 0x02;
const T_MEDIA: u8 = 0x03;
const T_DPI: u8 = 0x04;
const T_WHEEL: u8 = 0x05;
const T_FUNCTION: u8 = 0x06;
const T_MACRO: u8 = 0x07;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Mouse,
    Dpi,
    Media,
    Browser,
    Function,
    Shortcut,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Mouse => "Mouse",
            Group::Dpi => "DPI",
            Group::Media => "Multimedia",
            Group::Browser => "Browser and apps",
            Group::Function => "Functions",
            Group::Shortcut => "Shortcuts",
        }
    }
}

pub struct ActionInfo {
    pub id: u8,
    pub label: &'static str,
    pub group: Group,
    /// Wire triplet `(type, a, b)`.
    pub wire: [u8; 3],
}

const fn a(id: u8, label: &'static str, group: Group, wire: [u8; 3]) -> ActionInfo {
    ActionInfo { id, label, group, wire }
}

const fn media(id: u8, label: &'static str, group: Group, usage: u16) -> ActionInfo {
    // inferred: consumer-page usage sent little-endian in (a, b)
    a(id, label, group, [T_MEDIA, (usage & 0xFF) as u8, (usage >> 8) as u8])
}

const fn key(id: u8, label: &'static str, mods: u8, usage: u8) -> ActionInfo {
    a(id, label, Group::Shortcut, [T_KEYBOARD, mods, usage])
}

const CTRL: u8 = 1;
const ALT: u8 = 4;
const WIN: u8 = 8;

pub static ACTIONS: &[ActionInfo] = &[
    a(0x00, "Button Off", Group::Mouse, [0, 0, 0]),
    // inferred: mouse button bitmask L=1 R=2 M=4 Back=8 Fwd=16
    a(0x01, "Left Click", Group::Mouse, [T_MOUSE, 0x01, 0]),
    a(0x02, "Right Click", Group::Mouse, [T_MOUSE, 0x02, 0]),
    a(0x03, "Middle Button", Group::Mouse, [T_MOUSE, 0x04, 0]),
    a(0x04, "Forward", Group::Mouse, [T_MOUSE, 0x10, 0]),
    a(0x05, "Backward", Group::Mouse, [T_MOUSE, 0x08, 0]),
    a(0x09, "Double Click", Group::Mouse, [T_FUNCTION, 0x01, 0]),
    a(0x0A, "Fire Button", Group::Mouse, [T_FUNCTION, 0x02, 0]),
    a(0x0B, "Scroll Up", Group::Mouse, [T_WHEEL, 0x01, 0]),
    a(0x0C, "Scroll Down", Group::Mouse, [T_WHEEL, 0x02, 0]),
    a(0x0D, "Scroll to Left", Group::Mouse, [T_WHEEL, 0x03, 0]),
    a(0x0E, "Scroll to Right", Group::Mouse, [T_WHEEL, 0x04, 0]),
    a(0x06, "DPI Cycle", Group::Dpi, [T_DPI, 0x01, 0]),
    a(0x07, "DPI +", Group::Dpi, [T_DPI, 0x02, 0]),
    a(0x08, "DPI -", Group::Dpi, [T_DPI, 0x03, 0]),
    a(0x16, "Easy Aim", Group::Function, [T_FUNCTION, 0x03, 0]),
    a(0x17, "LED Loop", Group::Function, [T_FUNCTION, 0x04, 0]),
    media(0x40, "Media Player", Group::Media, 0x0183),
    media(0x41, "Play / Pause", Group::Media, 0x00CD),
    media(0x42, "Stop", Group::Media, 0x00B7),
    media(0x43, "Previous Track", Group::Media, 0x00B6),
    media(0x44, "Next Track", Group::Media, 0x00B5),
    media(0x45, "Volume +", Group::Media, 0x00E9),
    media(0x46, "Volume -", Group::Media, 0x00EA),
    media(0x47, "Mute", Group::Media, 0x00E2),
    media(0x50, "Browser Home", Group::Browser, 0x0223),
    media(0x51, "Browser Favorites", Group::Browser, 0x022A),
    media(0x52, "Browser Forward", Group::Browser, 0x0225),
    media(0x53, "Browser Backward", Group::Browser, 0x0224),
    media(0x54, "Browser Stop", Group::Browser, 0x0226),
    media(0x55, "Browser Refresh", Group::Browser, 0x0227),
    media(0x56, "Browser Search", Group::Browser, 0x0221),
    media(0x57, "Email", Group::Browser, 0x018A),
    media(0x58, "Calculator", Group::Browser, 0x0192),
    media(0x59, "My Computer", Group::Browser, 0x0194),
    // inferred: preset modifiers/keys are the standard Windows shortcuts; only Find/Close/Swap keys are observed
    key(0x20, "Cut", CTRL, 0x1B),
    key(0x21, "Copy", CTRL, 0x06),
    key(0x22, "Paste", CTRL, 0x19),
    key(0x23, "Open", CTRL, 0x12),
    key(0x24, "Save", CTRL, 0x16),
    key(0x25, "Find", CTRL, 0x09),
    key(0x26, "Redo", CTRL, 0x1C),
    key(0x27, "Undo", CTRL, 0x1D),
    key(0x28, "Select All", CTRL, 0x04),
    key(0x29, "Print", CTRL, 0x13),
    key(0x2A, "Close Window", ALT, 0x3D),
    key(0x2B, "Swap Windows", ALT, 0x2B),
    key(0x2C, "Show Desktop", WIN, 0x07),
    key(0x2D, "Run Command", WIN, 0x15),
    key(0x2E, "Lock PC", WIN, 0x0F),
    key(0x32, "Screen Capture", 0, 0x46),
];

pub fn info(id: u8) -> Option<&'static ActionInfo> {
    ACTIONS.iter().find(|x| x.id == id)
}

pub fn label(button: &ButtonConfig) -> String {
    match button.action_id {
        ACTION_SHORTCUT => format!("Shortcut: {}", super::keys::shortcut_label(button.shortcut_mods, button.shortcut_key)),
        ACTION_MACRO => match &button.macro_binding {
            Some(m) => format!("Macro: {}", m.name),
            None => "Macro: (none)".to_string(),
        },
        id => info(id).map_or("Left Click", |x| x.label).to_string(),
    }
}

/// `(type, a, b)` for button `index` (0-based). Unknown ids encode as zeros.
pub fn wire_triplet(button: &ButtonConfig, index: usize) -> [u8; 3] {
    match button.action_id {
        ACTION_SHORTCUT => [T_KEYBOARD, button.shortcut_mods, button.shortcut_key],
        // inferred: macro type byte (table b0 unknown), a = 0 and b = index + 1 are observed
        ACTION_MACRO => [T_MACRO, 0, (index + 1) as u8],
        id => info(id).map_or([0, 0, 0], |x| x.wire),
    }
}
