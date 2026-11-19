//! Device layer: transports, send rules, discovery and the event reader.

pub mod hid;
pub mod manager;
pub mod sender;

#[cfg(feature = "ble")]
pub mod ble;

#[cfg(test)]
mod sender_tests;

use crate::protocol::events::Event;
use std::fmt;

/// Connection type. Declaration order is the send priority (wired > dongle > BLE).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LinkKind {
    Wired,
    Dongle,
    Ble,
}

impl LinkKind {
    pub fn label(self) -> &'static str {
        match self {
            LinkKind::Wired => "Wired",
            LinkKind::Dongle => "2.4G",
            LinkKind::Ble => "Bluetooth",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceError(pub String);

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DeviceError {}

/// One raw write channel to the device. The packet is passed as-is: `buf[0]` is the command
/// and report ID (spec 2.1).
pub trait Transport: Send {
    fn kind(&self) -> LinkKind;
    fn write(&mut self, data: &[u8]) -> Result<(), DeviceError>;
    /// Periodic read for devices that report their battery only on request.
    fn poll(&mut self) -> Option<Event> {
        None
    }
}
