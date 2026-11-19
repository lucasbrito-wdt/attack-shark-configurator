//! Multi-model abstraction: a `Platform` (BK, XVI, V4), a static `ModelSpec` table per platform and
//! a `PlatformCodec` per platform. New platforms add a module with its table and codec, then register
//! both here (`models()` and `codec()`); nothing in `bk/` needs to change.

pub mod battery;
pub mod bk;
pub mod readback;
pub mod spec;
pub mod v4;
pub mod xvi;

use crate::device::LinkKind;
use crate::profile::Profile;
use crate::protocol::events::Event;
pub use readback::{DpiVerdict, ReadBack, ReadError, ReadFn, SettingsSpec};
pub use spec::{DeviceIds, DpiSpec, LightingSpec, ModelSpec};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Platform {
    /// VID 0x1D57 (families A, B and C1).
    Bk,
    /// VID 0x373E (R6, R5 Ultra). Implemented in `xvi/`.
    Xvi,
    /// VID 0x3151 (X3 PRO, R2, R3 on Driver v4). Implemented in `v4/`.
    V4,
}

impl Platform {
    pub fn id(self) -> &'static str {
        match self {
            Platform::Bk => "bk",
            Platform::Xvi => "xvi",
            Platform::V4 => "v4",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanError {
    /// The model's DPI encoding is not verified on hardware and `X3_ALLOW_UNVERIFIED=1` is not set.
    UnverifiedEncoding,
    /// No codec is registered for the model's platform.
    Unsupported,
}

/// Where packets go on a USB link of one PID: usage (page, usage) candidates per role, in preference
/// order, and whether config writes are output reports (`hid_write`) instead of feature reports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsbRoute {
    pub config: &'static [(u16, u16)],
    pub events: &'static [(u16, u16)],
    pub output_reports: bool,
}

/// Wire behavior of one platform. Pure: no I/O.
pub trait PlatformCodec: Sync {
    /// Packets for `new`, diffed per group against `prev` (`None` = every group), in send order.
    /// Refuses with `UnverifiedEncoding` when a DPI write would use an unverified encoding and
    /// `allow_unverified` is false.
    fn plan(&self, spec: &ModelSpec, prev: Option<&Profile>, new: &Profile, resync: bool, allow_unverified: bool)
    -> Result<Vec<Vec<u8>>, PlanError>;
    /// USB input report from the event interface.
    fn parse_report(&self, data: &[u8]) -> Option<Event>;
    /// BLE notification.
    fn parse_notification(&self, data: &[u8]) -> Option<Event>;
    fn usb_route(&self, pid: u16) -> UsbRoute;
    /// Split one packet into the writes the link needs (identity unless the link frames packets).
    fn frame(&self, pid: u16, pkt: &[u8]) -> Vec<Vec<u8>>;
    /// Model bound by the first input report on a shared receiver, if `pid` is one.
    fn detect_from_report(&self, pid: u16, report: &[u8]) -> Option<&'static ModelSpec>;
    /// True when `pkt` writes DPI with an encoding not verified on hardware: apply skips only these
    /// packets unless `X3_ALLOW_UNVERIFIED=1`. A codec that keeps the default (cannot tell) has its
    /// whole plan refused instead, through `plan(.., allow_unverified = false)`.
    fn is_unverified_dpi(&self, _spec: &ModelSpec, _pkt: &[u8]) -> bool {
        false
    }
    /// Highest polling rate (Hz) `link` carries when it is below the model's list (e.g. a wired cap).
    fn max_polling(&self, _spec: &ModelSpec, _link: LinkKind) -> Option<u16> {
        None
    }
    /// Feature writes the device confirms: after each SetFeature, GetFeature this many bytes
    /// (report ID included) until `is_ack`, retried; no ack = failed write.
    fn ack_reply_len(&self) -> Option<usize> {
        None
    }
    fn is_ack(&self, _request: &[u8], _reply: &[u8]) -> bool {
        true
    }
    /// Feature query polled every 2 s on devices with no battery event; the reply goes to `parse_battery_reply`.
    fn battery_request(&self) -> Option<Vec<u8>> {
        None
    }
    fn parse_battery_reply(&self, _reply: &[u8]) -> Option<Event> {
        None
    }
    /// Feature query sent when a link of `pid` opens, to resolve the model behind a shared PID.
    /// While it does not resolve, the model stays unknown (apply answers `unsupported_model`).
    fn identify_request(&self, _pid: u16) -> Option<Vec<u8>> {
        None
    }
    fn identify(&self, _reply: &[u8]) -> Option<&'static ModelSpec> {
        None
    }
    /// Optional settings the codec writes for `spec` (capabilities and patch validation follow it).
    fn settings(&self, _spec: &ModelSpec) -> SettingsSpec {
        SettingsSpec::default()
    }
    /// Read the device's current settings through `read` (feature report id, length). Read-only.
    fn read_back(&self, spec: &ModelSpec, _read: &mut ReadFn<'_>) -> Result<ReadBack, ReadError> {
        Err(ReadError::Unsupported(format!("{} has no read-back", spec.name)))
    }
    /// Decide, read-only, which DPI encoding the firmware stores (`DpiSpec::verified == false` models).
    fn verify_dpi(&self, spec: &ModelSpec, _read: &mut ReadFn<'_>) -> Result<DpiVerdict, ReadError> {
        Ok(DpiVerdict { format: None, firmware: None, reason: Some(format!("{} has no read-only DPI check", spec.name)) })
    }
}

pub fn codec(platform: Platform) -> Option<&'static dyn PlatformCodec> {
    match platform {
        Platform::Bk => Some(&bk::BkCodec),
        Platform::V4 => Some(&v4::V4Codec),
        Platform::Xvi => Some(&xvi::XviCodec),
    }
}

pub fn models() -> impl Iterator<Item = &'static ModelSpec> {
    bk::registry::MODELS.iter().map(|m| &m.spec).chain(v4::registry::MODELS.iter().map(|m| &m.spec)).chain(xvi::registry::MODELS.iter())
}

pub fn by_id(id: &str) -> Option<&'static ModelSpec> {
    models().find(|m| m.id == id)
}

/// Model used when nothing is connected and nothing was persisted: the X6 (BK, code 0x10).
pub fn default_model() -> &'static ModelSpec {
    by_id("x6").expect("registry holds x6")
}

/// `X3_ALLOW_UNVERIFIED=1` lets apply send DPI with an encoding not verified on hardware.
pub fn allow_unverified() -> bool {
    std::env::var("X3_ALLOW_UNVERIFIED").is_ok_and(|v| v == "1")
}

/// Every USB (vid, pid) to watch, with the link kind it represents. Deduplicated, table order.
pub fn usb_ids() -> Vec<(u16, u16, LinkKind)> {
    let mut out: Vec<(u16, u16, LinkKind)> = Vec::new();
    for m in models() {
        let ids = &m.ids;
        let links = [(ids.wired, LinkKind::Wired), (ids.dongle, LinkKind::Dongle), (ids.dongle_4k, LinkKind::Dongle)];
        for (pid, kind) in links {
            if let Some(pid) = pid
                && !out.iter().any(|(v, p, _)| *v == ids.vid && *p == pid)
            {
                out.push((ids.vid, pid, kind));
            }
        }
    }
    out
}

pub fn platform_of_vid(vid: u16) -> Option<Platform> {
    models().find(|m| m.ids.vid == vid).map(|m| m.platform)
}

/// Longest model name that starts `device_name` at a word boundary (e.g. "X6 Mouse2" -> X6).
/// Ties go to the first table entry.
pub fn by_name(device_name: &str) -> Option<&'static ModelSpec> {
    let name = device_name.trim().to_ascii_uppercase();
    let fits = |m: &&ModelSpec| {
        let model = m.name.to_ascii_uppercase();
        name.strip_prefix(&model).is_some_and(|rest| rest.is_empty() || rest.starts_with([' ', '-', '_']))
    };
    let mut best: Option<&'static ModelSpec> = None;
    for m in models().filter(fits) {
        if best.is_none_or(|b| m.name.len() > b.name.len()) {
            best = Some(m);
        }
    }
    best
}

/// USB detection by (VID, PID), with the product string as a hint when several models share the PID.
/// `None` for a shared receiver: the model is bound by its first input report (`detect_from_report`).
pub fn detect_usb(vid: u16, pid: u16, product: Option<&str>) -> Option<&'static ModelSpec> {
    let wired: Vec<&'static ModelSpec> = models().filter(|m| m.ids.vid == vid && m.ids.wired == Some(pid)).collect();
    match wired.as_slice() {
        [] => None,
        [one] => Some(one),
        many => product.and_then(by_name).filter(|m| many.contains(m)).or(Some(many[0])),
    }
}

/// BLE detection: the advertised name first, then the model code of the first notification.
pub fn detect_ble(name: Option<&str>, first_notification: Option<&[u8]>) -> Option<&'static ModelSpec> {
    name.and_then(by_name).filter(|m| m.ids.ble.is_some()).or_else(|| first_notification.and_then(bk::registry::by_ble_notification))
}

#[cfg(test)]
mod tests;
