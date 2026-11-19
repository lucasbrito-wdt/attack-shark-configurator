//! BK read-back (family-b 3.6, family-c C1.2): firmware 0x0B, then the split reads 0x06, 0x05, 0x04,
//! decoded with the same builders the apply uses (`readback::invert`). The 0x0A bulk read of firmware
//! other than 0x10xx and the 0x08 button map are not decoded.

use super::codec;
use super::frame::{self, CMD_VERSION};
use super::registry::{BkFamily, BkWire};
use crate::platform::readback::{ReadBack, ReadError, ReadFn, invert};
use crate::platform::ModelSpec;
use crate::profile::{DPI_STAGES, Profile};
use crate::protocol::{CMD_LIGHTING, CMD_POLLING, CMD_SENSOR};

const POLLING_LEN: u8 = 9;
const LIGHTING_LEN: u8 = 15;
pub(super) const SENSOR_LEN: u8 = 0x38;
/// Bytes covered by each packet's data (before the checksum).
const POLLING_DATA: std::ops::Range<usize> = 3..5;
const LIGHTING_DATA: std::ops::Range<usize> = 3..11;
const SENSOR_DATA: std::ops::Range<usize> = 3..50;

/// The reply must echo `[0] = id`, `[1] = len` (family-b 3.6).
pub(super) fn checked(read: &mut ReadFn<'_>, id: u8, len: u8) -> Result<Vec<u8>, ReadError> {
    let r = read(id, usize::from(len))?;
    if r.len() < usize::from(len) || r[0] != id || r[1] != len {
        return Err(ReadError::Decode(format!("0x{id:02X} reply has the wrong header or length ({} bytes)", r.len())));
    }
    Ok(r)
}

pub fn read_back(spec: &ModelSpec, wire: &BkWire, read: &mut ReadFn<'_>) -> Result<ReadBack, ReadError> {
    let mut out = ReadBack::default();
    // C1 answers per-command reads directly (C1.2); family B picks split or bulk by firmware
    if wire.family != BkFamily::C1 {
        out.firmware = frame::parse_version(&read(CMD_VERSION, 8)?);
        if let Some(v) = out.firmware.filter(|v| !frame::split_read(*v)) {
            return Err(ReadError::Decode(format!("firmware 0x{v:04X} answers the 0x0A bulk read, whose layout is not decoded")));
        }
    }
    let base = Profile::default();

    let polling = checked(read, CMD_POLLING, POLLING_LEN)?;
    let rates = spec.polling_rates.len() as u8;
    out.polling_index = invert(&|p| codec::build_polling(wire, p), &base, |p, v| p.polling_index = v, 0..rates, &polling, POLLING_DATA);

    let light = checked(read, CMD_LIGHTING, LIGHTING_LEN)?;
    let build = |p: &Profile| codec::build_lighting(wire, p);
    let field = |set: fn(&mut Profile, u8), base: &Profile| invert(&build, base, set, 0..=u8::MAX, &light, LIGHTING_DATA);
    // mode alone on byte 3: the brightness byte also changes with the mode
    out.light_mode = invert(&build, &base, |p, v| p.light_mode = v, 0..16, &light, 3..4);
    // brightness is only carried in some modes: decode it under the mode the device reports
    let lit = Profile { light_mode: out.light_mode.unwrap_or(base.light_mode), ..base.clone() };
    out.brightness = invert(&build, &lit, |p, v| p.brightness = v, 0..16, &light, LIGHTING_DATA);
    out.light_speed = invert(&build, &base, |p, v| p.light_speed = v, 0..16, &light, LIGHTING_DATA);
    out.deep_sleep_minutes = field(|p, v| p.deep_sleep_minutes = v, &base);
    out.sleep_minutes = field(|p, v| p.sleep_minutes = v, &base);
    out.debounce = field(|p, v| p.debounce = v, &base);
    out.light_color = light.get(6..9).and_then(|c| c.try_into().ok());

    let sensor = checked(read, CMD_SENSOR, SENSOR_LEN)?;
    let build = |p: &Profile| codec::build_sensor(spec, wire, p);
    let flag = |set: fn(&mut Profile, bool)| invert(&build, &base, set, [false, true], &sensor, SENSOR_DATA);
    out.ripple_control = flag(|p, v| p.ripple_control = v);
    out.angle_snap = flag(|p, v| p.angle_snap = v);
    out.motion_sync = flag(|p, v| p.motion_sync = v);
    if !spec.lod_options.is_empty() {
        out.lift_off = invert(&build, &base, |p, v| p.lift_off = v, 0..spec.lod_options.len() as u8, &sensor, SENSOR_DATA);
    }
    out.active_stage = invert(&build, &base, |p, v| p.active_stage = v, 0..DPI_STAGES as u8, &sensor, SENSOR_DATA);
    let mut tinted = base.clone();
    tinted.stage_colors.iter_mut().for_each(|c| *c = [0x12, 0x34, 0x56]);
    let colors = build(&base)[SENSOR_DATA] != build(&tinted)[SENSOR_DATA];
    for i in 0..DPI_STAGES {
        out.stage_enabled[i] = invert(&build, &base, |p, v| p.stage_enabled[i] = v, [false, true], &sensor, SENSOR_DATA);
        out.dpi_units[i] = invert(&build, &base, |p, v| p.dpi_units[i] = v, 0..=spec.dpi.max_units, &sensor, SENSOR_DATA);
        if colors {
            out.stage_colors[i] = sensor.get(25 + 3 * i..28 + 3 * i).and_then(|c| c.try_into().ok());
        }
    }
    Ok(out)
}
