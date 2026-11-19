//! Platform-neutral model description. Wire details that only one platform understands live in that
//! platform's own table (e.g. `bk::registry::BkWire`), keyed by `ModelSpec::id`.

use super::Platform;
use super::battery::BatterySpec;
use crate::model::{ButtonInfo, Capabilities, LightingMode, Range, dpi_from_units};
use crate::profile::BUTTON_COUNT;

#[derive(Debug, PartialEq, Eq)]
pub struct DeviceIds {
    /// VID of the wired and receiver links.
    pub vid: u16,
    pub wired: Option<u16>,
    /// Shared 2.4 GHz receiver (BK: 0xFA60).
    pub dongle: Option<u16>,
    /// 4K/8K receiver (BK: 0xFA65).
    pub dongle_4k: Option<u16>,
    /// (VID, PID) of the BLE HID device.
    pub ble: Option<(u16, u16)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DpiSpec {
    /// Highest slider unit `u` (BK: DPI = 50*u up to `threshold`, then `(u - threshold/2) * 100`).
    pub max_units: u16,
    pub threshold: Option<u16>,
    pub stages: u8,
    /// Short name of the wire encoding: `a | t | h | direct`.
    pub encoding: &'static str,
    /// False when the encoding conflicts between sources and is not settled on hardware.
    pub verified: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LightingSpec {
    pub modes: &'static [&'static str],
    /// The vendor UI shows the lighting panel.
    pub visible: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ModelSpec {
    /// Stable key, persisted as the last-used model.
    pub id: &'static str,
    pub name: &'static str,
    pub platform: Platform,
    /// BK: low byte of every event type (`type = category | code`) and `buf[1]` on the receiver.
    pub code: u16,
    pub ids: DeviceIds,
    pub dpi: DpiSpec,
    /// Hz, ascending. Every entry is a prefix of 125, 250, 500, 1000, 2000, 4000, 8000.
    pub polling_rates: &'static [u16],
    /// Lift-off options in mm; empty = no LOD setting.
    pub lod_options: &'static [u8],
    pub lighting: LightingSpec,
    /// Buttons to expose; the profile always holds 18 slots.
    pub buttons: &'static [ButtonInfo],
    pub macros: bool,
    pub battery: BatterySpec,
    /// Answers the 0xA0 read-back request.
    pub read_back: bool,
}

pub const ALL_POLLING_RATES: [u16; 7] = [125, 250, 500, 1000, 2000, 4000, 8000];

impl ModelSpec {
    pub fn dpi_max(&self) -> u32 {
        dpi_from_units(self.dpi.threshold, self.dpi.max_units)
    }

    pub fn capabilities(&self) -> Capabilities {
        let s = super::codec(self.platform).map(|c| c.settings(self)).unwrap_or_default();
        let range = |r: Option<(u8, u8)>| r.map(|(min, max)| Range { min: min.into(), max: max.into() });
        let lighting_mode = |(i, label): (usize, &&'static str)| {
            let id = i as u8;
            LightingMode { id, label, uses_brightness: s.brightness_modes.is_none_or(|m| m.contains(&id)), uses_color: None, uses_speed: None }
        };
        Capabilities {
            model_id: self.id,
            model_name: self.name,
            platform: self.platform.id(),
            model_code: self.code,
            max_dpi_stages: usize::from(self.dpi.stages),
            dpi_min: 50,
            dpi_max: self.dpi_max(),
            dpi_step: 50,
            dpi_high_step_from: self.dpi.threshold.map(|t| dpi_from_units(None, t)),
            dpi_encoding: self.dpi.encoding,
            dpi_verified: self.dpi.verified,
            polling_rates: self.polling_rates.to_vec(),
            lod_options: self.lod_options.to_vec(),
            lighting_modes: self.lighting.modes.iter().enumerate().map(lighting_mode).collect(),
            buttons: self.buttons.to_vec(),
            total_buttons: BUTTON_COUNT,
            supports_angle_snap: s.angle_snap,
            supports_ripple_control: s.ripple_control,
            supports_motion_sync: s.motion_sync,
            debounce: range(s.debounce),
            debounce_ms_per_unit: s.debounce_ms_per_unit,
            sleep_minutes: range(s.sleep_minutes),
            deep_sleep_minutes: range(s.deep_sleep_minutes),
            brightness: Range { min: 1, max: 8 },
            light_speed: Range { min: 4, max: 8 },
            max_macro_events: 50,
            supports_ble: cfg!(feature = "ble") && self.ids.ble.is_some(),
            supports_lighting: self.lighting.visible,
            supports_macros: self.macros,
            battery_over_ble: self.battery.over_ble,
            supports_read_back: self.read_back,
        }
    }
}
