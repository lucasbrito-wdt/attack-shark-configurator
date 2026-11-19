//! HID keyboard usage names and macro mouse codes.

pub const MOD_CTRL: u8 = 1;
pub const MOD_SHIFT: u8 = 2;
pub const MOD_ALT: u8 = 4;
pub const MOD_WIN: u8 = 8;

// inferred: mouse buttons inside macros use codes 0xF1..0xF5, verify with USB capture
pub const MOUSE_LEFT: u8 = 0xF1;
pub const MOUSE_RIGHT: u8 = 0xF2;
pub const MOUSE_MIDDLE: u8 = 0xF3;
pub const MOUSE_FORWARD: u8 = 0xF4;
pub const MOUSE_BACKWARD: u8 = 0xF5;

pub const MOUSE_KEYS: [(u8, &str); 5] = [
    (MOUSE_LEFT, "Left Click"),
    (MOUSE_RIGHT, "Right Click"),
    (MOUSE_MIDDLE, "Middle Click"),
    (MOUSE_FORWARD, "Forward"),
    (MOUSE_BACKWARD, "Backward"),
];

pub fn hid_label(usage: u8) -> String {
    const LETTERS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const DIGITS: &str = "1234567890";
    let fixed = match usage {
        0x28 => "Enter",
        0x29 => "Esc",
        0x2A => "Backspace",
        0x2B => "Tab",
        0x2C => "Space",
        0x2D => "-",
        0x2E => "=",
        0x2F => "[",
        0x30 => "]",
        0x31 => "\\",
        0x33 => ";",
        0x34 => "'",
        0x35 => "`",
        0x36 => ",",
        0x37 => ".",
        0x38 => "/",
        0x39 => "CapsLock",
        0x46 => "PrintScreen",
        0x47 => "ScrollLock",
        0x48 => "Pause",
        0x49 => "Insert",
        0x4A => "Home",
        0x4B => "PageUp",
        0x4C => "Delete",
        0x4D => "End",
        0x4E => "PageDown",
        0x4F => "Right",
        0x50 => "Left",
        0x51 => "Down",
        0x52 => "Up",
        0xE0 => "LCtrl",
        0xE1 => "LShift",
        0xE2 => "LAlt",
        0xE3 => "LWin",
        0xE4 => "RCtrl",
        0xE5 => "RShift",
        0xE6 => "RAlt",
        0xE7 => "RWin",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_string();
    }
    match usage {
        0x04..=0x1D => LETTERS[usize::from(usage - 0x04)..][..1].to_string(),
        0x1E..=0x27 => DIGITS[usize::from(usage - 0x1E)..][..1].to_string(),
        0x3A..=0x45 => format!("F{}", usage - 0x3A + 1),
        0x68..=0x73 => format!("F{}", usage - 0x68 + 13),
        0 => "(none)".to_string(),
        _ => MOUSE_KEYS
            .iter()
            .find(|(code, _)| *code == usage)
            .map_or_else(|| format!("0x{usage:02X}"), |(_, name)| (*name).to_string()),
    }
}

pub fn shortcut_label(mods: u8, key: u8) -> String {
    let mut parts: Vec<String> = [(MOD_CTRL, "Ctrl"), (MOD_SHIFT, "Shift"), (MOD_ALT, "Alt"), (MOD_WIN, "Win")]
        .iter()
        .filter(|(bit, _)| mods & bit != 0)
        .map(|(_, name)| (*name).to_string())
        .collect();
    parts.push(hid_label(key));
    parts.join("+")
}
