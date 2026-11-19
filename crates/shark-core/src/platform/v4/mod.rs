//! V4 platform (VID 0x3151, ATTACK SHARK Driver v4): X3 PRO, R2, R3. Driven by `registry::MODELS`.
//!
//! Supported: DPI stages, polling rate, and on class CH585 also debounce, angle snap, ripple and motion sync.
//! Unsupported (not decoded, family-c.md C3 unknowns), so absent from the capabilities: lighting (option nibble
//! and speed/brightness ranges unclear), button remap and macros (the whole key matrix must be rewritten with
//! unknown reserved slots), lift-off (silent height unit), sleep timers (unit), pairing (no command), battery
//! (no command on USB, no charge flag on 2.4G), the 2.4G dongle relay, Bluetooth, and the Pan1080 option param 0.
//! DPI is `verified: false`: the wire unit is not stated, so writes need `SHARK_ALLOW_UNVERIFIED=1`.

pub mod codec;
pub mod registry;

#[cfg(test)]
mod tests;

use super::{ModelSpec, PlanError, PlatformCodec, SettingsSpec, UsbRoute};
use crate::profile::Profile;
use crate::protocol::events::Event;

pub struct V4Codec;

// usage page 0xFFFF / usage 2 (C3.1), feature reports, report ID 0. The HID descriptor is unverified.
const ROUTE: UsbRoute = UsbRoute { config: &[(0xFFFF, 2)], events: &[], output_reports: false };

impl PlatformCodec for V4Codec {
    /// `resync` only marks everything dirty: the vendor reset (0x0E) is a factory reset and is never sent.
    fn plan(&self, spec: &ModelSpec, prev: Option<&Profile>, new: &Profile, resync: bool, allow_unverified: bool)
    -> Result<Vec<Vec<u8>>, PlanError> {
        let model = registry::by_spec(spec).ok_or(PlanError::Unsupported)?;
        let old = if resync { None } else { prev.map(|p| codec::build_all(model, p)) };
        let packets: Vec<Vec<u8>> =
            codec::build_all(model, new).into_iter().filter(|pkt| old.as_ref().is_none_or(|o| !o.contains(pkt))).collect();
        if !spec.dpi.verified && !allow_unverified && packets.iter().any(|p| codec::is_dpi(p)) {
            return Err(PlanError::UnverifiedEncoding);
        }
        Ok(packets)
    }

    // no device-to-host events on USB (C3.5)
    fn parse_report(&self, _: &[u8]) -> Option<Event> {
        None
    }

    fn parse_notification(&self, _: &[u8]) -> Option<Event> {
        None
    }

    fn usb_route(&self, _: u16) -> UsbRoute {
        ROUTE
    }

    /// The feature report carries report ID 0 in front of the 64-byte payload (C3.2).
    fn frame(&self, _: u16, pkt: &[u8]) -> Vec<Vec<u8>> {
        vec![[&[0u8][..], pkt].concat()]
    }

    /// No shared receiver is supported; the model comes from cmd 0x8F (`registry::by_device_id`).
    fn detect_from_report(&self, _: u16, _: &[u8]) -> Option<&'static ModelSpec> {
        None
    }

    fn is_unverified_dpi(&self, spec: &ModelSpec, pkt: &[u8]) -> bool {
        !spec.dpi.verified && codec::is_dpi(pkt)
    }

    /// PIDs shared by several models (0x4026, 0x5043, 0x503A) are resolved with cmd 0x8F.
    fn identify_request(&self, pid: u16) -> Option<Vec<u8>> {
        let users = registry::MODELS.iter().filter(|m| m.spec.ids.wired == Some(pid) || m.spec.ids.dongle == Some(pid)).count();
        (users > 1).then(codec::build_get_id)
    }

    /// The GetFeature reply starts with report ID 0, then `8F id[4]`.
    fn identify(&self, reply: &[u8]) -> Option<&'static ModelSpec> {
        registry::by_device_id(codec::parse_device_id(reply.get(1..)?)?)
    }

    /// CH585 has one command each for debounce (ms), angle snap, ripple and motion sync; Pan1080 none.
    fn settings(&self, spec: &ModelSpec) -> SettingsSpec {
        match registry::by_spec(spec) {
            Some(m) if m.class == registry::V4Class::Ch585 => SettingsSpec {
                angle_snap: true,
                ripple_control: true,
                motion_sync: true,
                // inferred: C3.3 gives no debounce range; the profile's 2..=25 is kept, sent as raw ms
                debounce: Some((2, 25)),
                debounce_ms_per_unit: 1,
                ..SettingsSpec::default()
            },
            _ => SettingsSpec::default(),
        }
    }
}
