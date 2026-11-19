//! Device -> host events (PROTOCOL.md section 4, family-b section 4, family-c C1.7).
//!
//! `type = category | model_code` (family-b 4.1): the high byte is the category, the low byte the
//! model code (0x10 for the X3/X6/R1). Family A's literal types (`0x1000`, `0x7000`...) are the
//! same categories with code 0.

pub const EV_DPI_STAGE: u16 = 0x1000;
pub const EV_POLLING: u16 = 0x2000;
pub const EV_HEARTBEAT: u16 = 0x4000;
/// C1 second battery category.
pub const EV_BATTERY_C1: u16 = 0x4100;
pub const EV_ACK: u16 = 0x5000;
pub const EV_BATTERY: u16 = 0x6000;
pub const EV_LIGHT_MODE: u16 = 0x7000;
pub const EV_SAVE: u16 = 0x8000;

/// Accept filter of X3.exe (spec 4.2): only types `0x1000..=0x9000` reach the handler.
pub fn type_accepted(kind: u16) -> bool {
    (0x1000..=0x9000).contains(&kind)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Active DPI stage changed on the device, 1..=8.
    DpiStage(u8),
    /// Raw polling value. Domain conflicts with the 0..3 index of cmd 0x06 (spec unknown #10).
    Polling(u16),
    /// Battery + heartbeat (`0x40xx`, C1 also `0x41xx`), raw: `state` = value low byte,
    /// `level` = value high byte. Decode with `platform::battery::decode`.
    Heartbeat { state: u8, level: u8 },
    Ack(u16),
    Battery(u8),
    LightMode(u8),
    Save(u16),
    /// Raw BLE Battery Service byte (0x2A19). 0..=100 is a level; anything else is not decoded.
    BasBattery(u8),
    /// Category inside the accept filter that no spec defines. Logged by the caller, never acted on.
    Unknown { kind: u16, value: u16 },
}

/// Interpret a `(type, value)` pair. Types outside the accept filter produce nothing.
pub fn parse_pair(kind: u16, value: u16) -> Option<Event> {
    if !type_accepted(kind) {
        return None;
    }
    let lo = (value & 0xFF) as u8;
    match kind & 0xFF00 {
        EV_DPI_STAGE => Some(Event::DpiStage(lo)),
        EV_POLLING => Some(Event::Polling(value)),
        EV_HEARTBEAT | EV_BATTERY_C1 => Some(Event::Heartbeat { state: lo, level: (value >> 8) as u8 }),
        EV_ACK => Some(Event::Ack(value)),
        EV_BATTERY => Some(Event::Battery(lo)),
        // family B/C1 (code != 0) carry the mode in the high nibble; family A's literal 0x7000 sends it raw
        EV_LIGHT_MODE => Some(Event::LightMode(if kind & 0xFF == 0 { lo } else { (lo >> 4) & 0x0F })),
        EV_SAVE => Some(Event::Save(value)),
        _ => Some(Event::Unknown { kind, value }),
    }
}

/// BLE notification (spec 4.3.3): `(type = Data[0..1], value = Data[2..3])` first; if the type
/// fails the accept filter, the swapped order is tried. `Data[4..]` is ignored.
pub fn parse_ble(data: &[u8]) -> Option<Event> {
    let [a0, a1, b0, b1, ..] = *data else { return None };
    let (first, second) = (u16::from_le_bytes([a0, a1]), u16::from_le_bytes([b0, b1]));
    if type_accepted(first) {
        parse_pair(first, second)
    } else {
        parse_pair(second, first)
    }
}

/// USB input report (spec 4.3.2): `03 TT TT VV VV ...`, little-endian, `b[5..]` ignored.
pub fn parse_input_report(data: &[u8]) -> Option<Event> {
    let [0x03, t0, t1, v0, v1, ..] = *data else { return None };
    parse_pair(u16::from_le_bytes([t0, t1]), u16::from_le_bytes([v0, v1]))
}

/// Debug aid: with `X3_LOG_EVENTS=1` every raw input report and BLE notification goes to stderr.
pub fn log_event_bytes(source: &str, bytes: &[u8]) {
    if log_enabled() {
        let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02X}")).collect();
        eprintln!("[{source}] {}", hex.join(" "));
    }
}

static FORCED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Turn on the raw event log regardless of `X3_LOG_EVENTS` (used by `x3-probe`).
pub fn force_event_log() {
    FORCED.store(true, std::sync::atomic::Ordering::Relaxed);
}

fn log_enabled() -> bool {
    use std::sync::OnceLock;
    static ON: OnceLock<bool> = OnceLock::new();
    FORCED.load(std::sync::atomic::Ordering::Relaxed) || *ON.get_or_init(|| std::env::var_os("X3_LOG_EVENTS").is_some())
}

/// Free-form diagnostic line, printed only with `X3_LOG_EVENTS=1`.
pub fn log_line(msg: &str) {
    if log_enabled() {
        eprintln!("{msg}");
    }
}
