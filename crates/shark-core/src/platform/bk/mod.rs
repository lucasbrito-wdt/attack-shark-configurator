//! BK platform (VID 0x1D57): one codec for families A, B and C1, driven by `registry::MODELS`.

pub mod codec;
pub mod frame;
mod read;
pub mod registry;
pub mod verify;

#[cfg(test)]
mod dpi_tables;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_events;
#[cfg(test)]
mod tests_read;

use super::{DpiVerdict, ModelSpec, PlanError, PlatformCodec, ReadBack, ReadError, ReadFn, SettingsSpec, UsbRoute};
use crate::profile::Profile;
use crate::protocol::apply::plan_groups;
use crate::protocol::events::{Event, parse_ble, parse_input_report};
use crate::protocol::{CMD_MACRO, CMD_SENSOR};
use registry::{BkDpi, BkFamily, PID_DONGLE_4K, PID_X3_PRO_WIRED, TableStyle};

pub struct BkCodec;

const LEGACY_ROUTE: UsbRoute =
    UsbRoute { config: &[(0x000B, 0), (0xFF00, 1)], events: &[(0x000A, 0), (0xFF01, 2)], output_reports: false };
// unknown #7/#9: which FA65 / X3 PRO collection takes the framed output report; every candidate is tried in order
const FRAMED_ROUTE: UsbRoute =
    UsbRoute { config: &[(0xFF01, 2), (0xFF00, 2), (0xFF00, 1)], events: &[(0xFF00, 1), (0x000A, 0)], output_reports: true };

impl PlatformCodec for BkCodec {
    fn plan(&self, spec: &ModelSpec, prev: Option<&Profile>, new: &Profile, resync: bool, allow_unverified: bool)
    -> Result<Vec<Vec<u8>>, PlanError> {
        let wire = registry::by_spec(spec).ok_or(PlanError::Unsupported)?.wire;
        // C1 has no 0x0C (it is "active profile" there); its reset is a full rewrite
        let resync = resync && wire.family != BkFamily::C1;
        let packets = plan_groups(
            prev,
            new,
            resync,
            [
                &|p| codec::build_sensor(spec, &wire, p),
                &|p| codec::build_lighting(&wire, p),
                &|p| codec::build_polling(&wire, p),
                &|p| codec::build_button_map(spec, &wire, p),
            ],
            spec.macros,
        );
        if !allow_unverified && packets.iter().any(|p| self.is_unverified_dpi(spec, p)) {
            return Err(PlanError::UnverifiedEncoding);
        }
        Ok(packets)
    }

    fn parse_report(&self, data: &[u8]) -> Option<Event> {
        parse_input_report(data)
    }

    fn parse_notification(&self, data: &[u8]) -> Option<Event> {
        // A0/A1/A2/A9 are read-back replies (family-b 4.3), not events
        if matches!(data.first(), Some(0xA0 | 0xA1 | 0xA2 | 0xA9)) {
            return None;
        }
        parse_ble(data)
    }

    fn usb_route(&self, pid: u16) -> UsbRoute {
        if matches!(pid, PID_DONGLE_4K | PID_X3_PRO_WIRED) { FRAMED_ROUTE } else { LEGACY_ROUTE }
    }

    /// 4K receiver and X3 PRO wire: framed output reports. Macros there use the undecoded 111-byte
    /// format, so they produce no frames (the write fails instead of sending a wrong packet).
    fn frame(&self, pid: u16, pkt: &[u8]) -> Vec<Vec<u8>> {
        match pid {
            _ if pkt.first() == Some(&CMD_MACRO) && self.usb_route(pid).output_reports => Vec::new(),
            PID_DONGLE_4K => frame::frame_4k(pkt, 0x41),
            PID_X3_PRO_WIRED => frame::frame_4k(pkt, 0x40),
            _ => vec![pkt.to_vec()],
        }
    }

    fn is_unverified_dpi(&self, spec: &ModelSpec, pkt: &[u8]) -> bool {
        if pkt.first() != Some(&CMD_SENSOR) {
            return false;
        }
        // family-b unknown #4: the variant T multiplier bits above 10000 DPI are copied, not understood.
        // Any stage with u > 200 sets a [6]/[7] bit or a [16..23] half-step flag.
        let t_high = registry::by_spec(spec).is_some_and(|m| matches!(m.wire.dpi, BkDpi::Table(_)))
            && pkt.len() >= 24
            && (pkt[6] | pkt[7] != 0 || pkt[16..24].iter().any(|&f| f != 0));
        !spec.dpi.verified || t_high
    }

    fn detect_from_report(&self, pid: u16, report: &[u8]) -> Option<&'static ModelSpec> {
        match report {
            [0x03, code, ..] => registry::by_receiver_code(pid, *code),
            _ => None,
        }
    }

    /// Sensor flags per 0x04 layout (`codec::build_sensor`); debounce, sleep and deep sleep ride on 0x05.
    fn settings(&self, spec: &ModelSpec) -> SettingsSpec {
        let Some(m) = registry::by_spec(spec) else { return SettingsSpec::default() };
        let (angle_snap, ripple_control, motion_sync) = match m.wire.dpi {
            BkDpi::A { .. } | BkDpi::High { .. } | BkDpi::Direct { paw3395: true } => (true, true, true),
            BkDpi::Table(TableStyle::R1) | BkDpi::Direct { paw3395: false } => (true, true, false),
            BkDpi::Table(TableStyle::V3) => (true, false, true),
        };
        SettingsSpec {
            angle_snap,
            ripple_control,
            motion_sync,
            debounce: Some((2, 25)),
            // inferred: the vendor labels the raw value as 2 x value ms (PROTOCOL.md 4, C1 sends keyRespond / 2)
            debounce_ms_per_unit: 2,
            sleep_minutes: Some((1, 60)),
            deep_sleep_minutes: Some((1, 60)),
            brightness_modes: codec::brightness_modes(m.wire.family),
        }
    }

    fn read_back(&self, spec: &ModelSpec, read: &mut ReadFn<'_>) -> Result<ReadBack, ReadError> {
        match registry::by_spec(spec) {
            Some(m) if spec.read_back => read::read_back(spec, &m.wire, read),
            _ => Err(ReadError::Unsupported(format!("{} has no read-back", spec.name))),
        }
    }

    /// Family A only (u-1 vs the R1 table, family-b unknown #1): firmware 0x0B, then the stored 0x04.
    fn verify_dpi(&self, spec: &ModelSpec, read: &mut ReadFn<'_>) -> Result<DpiVerdict, ReadError> {
        if registry::by_spec(spec).is_none_or(|m| m.wire.family != BkFamily::A) {
            let reason = format!("{} has no read-only DPI check", spec.name);
            return Ok(DpiVerdict { format: None, firmware: None, reason: Some(reason) });
        }
        let firmware = frame::parse_version(&read(frame::CMD_VERSION, 8)?);
        if let Some(v) = firmware.filter(|v| !frame::split_read(*v)) {
            return Err(ReadError::Decode(format!("firmware 0x{v:04X} answers the 0x0A bulk read, whose layout is not decoded")));
        }
        let sensor = read::checked(read, CMD_SENSOR, read::SENSOR_LEN)?;
        Ok(DpiVerdict { firmware, ..verify::judge(spec, &sensor) })
    }
}
