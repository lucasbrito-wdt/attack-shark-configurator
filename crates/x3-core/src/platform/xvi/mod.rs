//! XVI platform (VID 0x373E): R6 (family E) and R5 Ultra (family C2). Feature reports, 65 bytes,
//! report ID 0, no checksum; battery is polled (see `codec::battery_request`).

pub mod codec;
pub mod registry;

#[cfg(test)]
mod tests;

use super::{ModelSpec, PlanError, PlatformCodec, SettingsSpec, UsbRoute};
use crate::device::LinkKind;
use crate::profile::Profile;
use crate::protocol::events::Event;

pub struct XviCodec;

// unknown: usage page / usage of the command and event interfaces (family-e 2.1, family-c C2.1); every
// vendor-page candidate is tried in order. The command interface is the one with a 65-byte feature report.
const ROUTE: UsbRoute = UsbRoute { config: &[(0xFF00, 0), (0xFF00, 1), (0xFF01, 0)], events: &[(0xFF00, 1), (0xFF01, 0)], output_reports: false };

impl PlatformCodec for XviCodec {
    fn plan(&self, spec: &ModelSpec, prev: Option<&Profile>, new: &Profile, resync: bool, allow_unverified: bool)
    -> Result<Vec<Vec<u8>>, PlanError> {
        codec::plan(spec, prev, new, resync, allow_unverified)
    }

    /// Events are not mapped yet: the DPI-stage base (0 or 1) is unconfirmed and link / LOD / polling
    /// events have no `Event` variant (family-e section 5, family-c C2.4). Nothing is guessed.
    fn parse_report(&self, _data: &[u8]) -> Option<Event> {
        None
    }

    fn parse_notification(&self, _data: &[u8]) -> Option<Event> {
        None
    }

    fn usb_route(&self, _pid: u16) -> UsbRoute {
        ROUTE
    }

    fn frame(&self, _pid: u16, pkt: &[u8]) -> Vec<Vec<u8>> {
        vec![pkt.to_vec()]
    }

    /// Each dongle PID maps to one model, so any report binds it.
    fn detect_from_report(&self, pid: u16, _report: &[u8]) -> Option<&'static ModelSpec> {
        registry::by_dongle_pid(pid)
    }

    /// Wired is capped at 1000 Hz on the device (family-e section 4).
    fn max_polling(&self, _spec: &ModelSpec, link: LinkKind) -> Option<u16> {
        (link == LinkKind::Wired).then_some(1000)
    }

    /// SetFeature, 30 ms, GetFeature, up to 10 tries until `[1] == 0xA1` (family-e 2.2).
    fn ack_reply_len(&self) -> Option<usize> {
        Some(65)
    }

    /// DPI stages (cat 1 cmd 1) and active stage (cat 1 cmd 2).
    fn is_unverified_dpi(&self, spec: &ModelSpec, pkt: &[u8]) -> bool {
        !spec.dpi.verified && pkt.get(5) == Some(&1) && matches!(pkt.get(6), Some(1 | 2))
    }

    fn is_ack(&self, request: &[u8], reply: &[u8]) -> bool {
        codec::is_ack(request, reply)
    }

    fn battery_request(&self) -> Option<Vec<u8>> {
        Some(codec::battery_request())
    }

    fn parse_battery_reply(&self, reply: &[u8]) -> Option<Event> {
        codec::parse_battery(reply)
    }

    /// `codec::options`: angle snap, ripple, motion sync, sleep in seconds (15..=1800 s, sent as minutes * 60).
    /// Debounce is not sent (its command collides with LOD).
    fn settings(&self, _spec: &ModelSpec) -> SettingsSpec {
        SettingsSpec { angle_snap: true, ripple_control: true, motion_sync: true, sleep_minutes: Some((1, 30)), ..SettingsSpec::default() }
    }
}
