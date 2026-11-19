//! Pure packet builders, checksum, macro chunking and event parser. No I/O.

pub mod actions;
pub mod apply;
pub mod events;
pub mod keys;
pub mod macros;
pub mod packets;

#[cfg(test)]
mod tests;

pub const VID: u16 = 0x1D57;
pub const PID_WIRED: u16 = 0xFA61;
pub const PID_DONGLE: u16 = 0xFA60;

pub const CMD_SENSOR: u8 = 0x04;
pub const CMD_LIGHTING: u8 = 0x05;
pub const CMD_POLLING: u8 = 0x06;
pub const CMD_BUTTONS: u8 = 0x08;
pub const CMD_MACRO: u8 = 0x09;
pub const CMD_RESYNC: u8 = 0x0C;

/// HID usage page of the feature/config interface (spec 1.2, observed).
pub const USAGE_PAGE_CONFIG: u16 = 0x000B;
/// HID usage page of the event/input interface (spec 1.2, observed).
pub const USAGE_PAGE_EVENTS: u16 = 0x000A;

/// Plain unsigned 16-bit byte sum (spec 2.1).
pub fn checksum16(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0u16, |acc, b| acc.wrapping_add(u16::from(*b)))
}

/// Store the big-endian checksum of `buf[first..=last]` at `buf[last + 1..=last + 2]`.
pub(crate) fn seal(buf: &mut [u8], first: usize, last: usize) {
    let sum = checksum16(&buf[first..=last]);
    buf[last + 1] = (sum >> 8) as u8;
    buf[last + 2] = (sum & 0xFF) as u8;
}
