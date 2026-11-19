//! Capabilities exposed to the UI, built from the detected `platform::ModelSpec`, plus the DPI
//! unit conversions every BK model shares.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LightingMode {
    pub id: u8,
    pub label: &'static str,
    /// The codec sends the brightness in this mode (otherwise a fixed value).
    pub uses_brightness: bool,
    /// Color and speed are sent in every mode; whether the mode shows them is not decoded (`None`).
    pub uses_color: Option<bool>,
    pub uses_speed: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonInfo {
    /// 0-based index into the profile button list.
    pub id: u8,
    pub label: &'static str,
    /// `left | right | middle | side-back | side-forward`
    pub position: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub min: u32,
    pub max: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub model_id: &'static str,
    pub model_name: &'static str,
    /// `bk | xvi | v4`
    pub platform: &'static str,
    pub model_code: u16,
    pub max_dpi_stages: usize,
    pub dpi_min: u32,
    pub dpi_max: u32,
    /// Step up to `dpi_high_step_from`; 100 above it.
    pub dpi_step: u32,
    pub dpi_high_step_from: Option<u32>,
    /// `a | t | h | direct`
    pub dpi_encoding: &'static str,
    /// False: DPI writes are refused unless `SHARK_ALLOW_UNVERIFIED=1`.
    pub dpi_verified: bool,
    /// Hz.
    pub polling_rates: Vec<u16>,
    /// Lift-off distance options in mm; empty = not adjustable.
    pub lod_options: Vec<u8>,
    pub lighting_modes: Vec<LightingMode>,
    /// Buttons the vendor UI exposes; the profile holds `total_buttons` slots.
    pub buttons: Vec<ButtonInfo>,
    pub total_buttons: usize,
    pub supports_angle_snap: bool,
    pub supports_ripple_control: bool,
    pub supports_motion_sync: bool,
    /// Raw debounce value range; `None` = the model's codec does not write it.
    pub debounce: Option<Range>,
    /// Milliseconds per raw debounce unit (BK 2, inferred from the vendor label; V4 1).
    pub debounce_ms_per_unit: u8,
    /// `None` = not written for this model.
    pub sleep_minutes: Option<Range>,
    pub deep_sleep_minutes: Option<Range>,
    pub brightness: Range,
    pub light_speed: Range,
    /// Max wire macro pairs (key events plus long-delay fillers).
    pub max_macro_events: usize,
    pub supports_ble: bool,
    /// The vendor UI shows the lighting panel; the packet is sent either way.
    pub supports_lighting: bool,
    pub supports_macros: bool,
    /// False: the vendor discards battery events over BLE for this model.
    pub battery_over_ble: bool,
    pub supports_read_back: bool,
}

/// BK slider units to DPI: `50*u` up to `threshold`, then `(u - threshold/2) * 100`.
pub fn dpi_from_units(threshold: Option<u16>, units: u16) -> u32 {
    let u = u32::from(units);
    match threshold.map(u32::from) {
        Some(t) if u > t => (u - t / 2) * 100,
        _ => u * 50,
    }
}

/// Inverse of `dpi_from_units`; `None` when `dpi` is off the 50 / 100 grid.
pub fn units_from_dpi(threshold: Option<u16>, dpi: u32) -> Option<u16> {
    let units = match threshold.map(u32::from) {
        Some(t) if dpi > t * 50 => dpi.is_multiple_of(100).then(|| dpi / 100 + t / 2),
        _ => dpi.is_multiple_of(50).then_some(dpi / 50),
    }?;
    u16::try_from(units).ok()
}

impl Capabilities {
    fn threshold(&self) -> Option<u16> {
        self.dpi_high_step_from.and_then(|d| u16::try_from(d / 50).ok())
    }

    pub fn dpi_from_units(&self, units: u16) -> u32 {
        dpi_from_units(self.threshold(), units)
    }

    /// Slider units for `dpi`, or `None` when it is out of range or off the grid.
    pub fn units_from_dpi(&self, dpi: u32) -> Option<u16> {
        if !(self.dpi_min..=self.dpi_max).contains(&dpi) {
            return None;
        }
        units_from_dpi(self.threshold(), dpi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{by_id, default_model};

    #[test]
    fn capabilities_json_keys_and_values() {
        let v = serde_json::to_value(default_model().capabilities()).unwrap();
        let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "batteryOverBle", "brightness", "buttons", "debounce", "debounceMsPerUnit", "deepSleepMinutes", "dpiEncoding",
                "dpiHighStepFrom", "dpiMax", "dpiMin", "dpiStep", "dpiVerified", "lightSpeed", "lightingModes", "lodOptions",
                "maxDpiStages", "maxMacroEvents", "modelCode", "modelId", "modelName", "platform", "pollingRates", "sleepMinutes",
                "supportsAngleSnap", "supportsBle", "supportsLighting", "supportsMacros", "supportsMotionSync", "supportsReadBack",
                "supportsRippleControl", "totalButtons",
            ]
        );
        assert_eq!((v["modelName"].as_str(), v["platform"].as_str(), v["modelCode"].as_u64()), (Some("X6"), Some("bk"), Some(0x10)));
        assert_eq!((v["dpiMax"].as_u64(), v["maxDpiStages"].as_u64(), v["dpiVerified"].as_bool()), (Some(26000), Some(8), Some(false)));
        assert_eq!(v["buttons"][3]["position"], "side-back");
        assert_eq!(v["debounce"]["min"], 2);
        assert_eq!(v["lightingModes"][1], serde_json::json!({"id": 1, "label": "Static", "usesBrightness": true, "usesColor": null, "usesSpeed": null}));
        assert_eq!(v["lightingModes"][2]["usesBrightness"], false);
    }

    #[test]
    fn dpi_formula_matches_every_threshold() {
        assert_eq!((dpi_from_units(None, 520), dpi_from_units(None, 1)), (26000, 50));
        assert_eq!((dpi_from_units(Some(200), 200), dpi_from_units(Some(200), 201)), (10000, 10100));
        assert_eq!(dpi_from_units(Some(200), 350), 25000);
        assert_eq!((dpi_from_units(Some(600), 720), dpi_from_units(Some(520), 660)), (42000, 40000));
        for t in [None, Some(200), Some(520), Some(600)] {
            for u in 1..=720 {
                assert_eq!(units_from_dpi(t, dpi_from_units(t, u)), Some(u), "threshold {t:?} units {u}");
            }
        }
        assert_eq!(units_from_dpi(Some(200), 10150), None);
        let caps = by_id("r1").unwrap().capabilities();
        assert_eq!((caps.dpi_max, caps.dpi_high_step_from, caps.units_from_dpi(18100)), (18000, Some(10000), None));
    }
}
