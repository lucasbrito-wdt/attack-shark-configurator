//! V4 model table (VID 0x3151, docs/protocol/family-c.md C3.1). The vendor never keys a model on PID
//! (0x4026, 0x5043 and 0x503A are each shared): the runtime device id from command 0x8F picks the
//! entry, and the entry's class picks the command table (C3.3). One entry per (model, class).

use crate::platform::Platform;
use crate::platform::battery::{BatteryScale, BatterySpec, ChargeRule};
use crate::platform::spec::{DeviceIds, DpiSpec, LightingSpec, ModelSpec};

pub const VID: u16 = 0x3151;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum V4Class {
    /// CommonMSCH585 (newer firmware): one command per setting.
    Ch585,
    /// CommonMsPan1080 (older firmware): most settings live inside the 64-byte option param 0.
    Pan1080,
}

#[derive(Debug, PartialEq, Eq)]
pub struct V4Model {
    pub spec: ModelSpec,
    pub class: V4Class,
    /// Runtime ids (cmd 0x8F) that bind to this entry.
    pub device_ids: &'static [u32],
}

const R4: [u16; 4] = [125, 250, 500, 1000];
const R7: [u16; 7] = [125, 250, 500, 1000, 2000, 4000, 8000];

// Unsupported on every V4 model (see mod.rs): no lighting, no key matrix, no macros, no LOD.
const fn model(id: &'static str, name: &'static str, pid: u16, class: V4Class, device_ids: &'static [u32], max_units: u16, rates: &'static [u16]) -> V4Model {
    V4Model {
        spec: ModelSpec {
            id,
            name,
            platform: Platform::V4,
            code: 0,
            ids: DeviceIds { vid: VID, wired: Some(pid), dongle: None, dongle_4k: None, ble: None },
            // wire value presumed raw DPI (C3.3 says "X LE16", no unit): unverified, like BK variant direct
            dpi: DpiSpec { max_units, threshold: None, stages: 6, encoding: "direct", verified: false },
            polling_rates: rates,
            lod_options: &[],
            lighting: LightingSpec { modes: &[], visible: false },
            buttons: &[],
            macros: false,
            // no battery command exists on USB (C3.5); the field only has to be well-formed
            battery: BatterySpec { scale: BatteryScale::Percent, rule: ChargeRule::StateTwoOrThree, over_ble: false },
            read_back: false,
        },
        class,
        device_ids,
    }
}

use V4Class::{Ch585, Pan1080};

// DPI max: 26000 = 520 units, 42000 = 840 units (C3.1). Wired rate: 8000 only on PIDs 0x402A / 0x503A (C3.3).
// Pan1080 is capped at 1000 Hz: 2K/4K/8K go through option param 0 (0x53), a read-modify-write of 64 bytes whose
// layout is only partly known (C3.3), so it is not planned.
// inferred: R3 3309/3310 and 3485 list no DPI max (C3.1); 26000 follows R3 1643 and the PAW3395 sensor name.
pub static MODELS: [V4Model; 7] = [
    model("x3-pro-v4", "X3 PRO (v4)", 0x503A, Ch585, &[3015], 520, &R7),
    model("r2-v4", "R2 (v4)", 0x5043, Ch585, &[3016], 840, &R4),
    model("r3-v4", "R3 (v4)", 0x5043, Ch585, &[3309, 3310], 520, &R4),
    model("r3-v4-newest", "R3 (v4 newest)", 0x503A, Ch585, &[3485], 520, &R4),
    model("x3-pro-v4-legacy", "X3 PRO (v4 legacy)", 0x402A, Pan1080, &[1660], 520, &R4),
    model("r2-v4-legacy", "R2 (v4 legacy)", 0x4026, Pan1080, &[1893, 3009], 840, &R4),
    model("r3-v4-legacy", "R3 (v4 legacy)", 0x4026, Pan1080, &[1643], 520, &R4),
];

pub fn by_spec(spec: &ModelSpec) -> Option<&'static V4Model> {
    MODELS.iter().find(|m| m.spec.id == spec.id)
}

/// Model for the u32 read with command 0x8F. `None` for unknown ids and for 0xFFFFFFFF (the vendor then
/// falls back to a PID table, C3.1, which this crate does not mirror).
pub fn by_device_id(id: u32) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|m| m.device_ids.contains(&id)).map(|m| &m.spec)
}
