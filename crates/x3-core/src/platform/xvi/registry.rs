//! XVI model table (VID 0x373E): R6 (family-e.md section 1) and R5 Ultra (family-c.md C2.1).
//! Both speak the same protocol; only the PIDs differ.

use crate::model::ButtonInfo;
use crate::platform::Platform;
use crate::platform::battery::{BatteryScale, BatterySpec, ChargeRule};
use crate::platform::spec::{ALL_POLLING_RATES, DeviceIds, DpiSpec, LightingSpec, ModelSpec};

pub const VID: u16 = 0x373E;

const fn button(id: u8, label: &'static str, position: &'static str) -> ButtonInfo {
    ButtonInfo { id, label, position }
}

/// Profile slots 0, 1, 2, 6, 7 = wire ButtonIDs 1, 2, 3, 4, 5 (family-e 3.5); see `codec::wire_button`.
const BUTTONS: [ButtonInfo; 5] = [
    button(0, "Left button", "left"),
    button(1, "Right button", "right"),
    button(2, "Middle button", "middle"),
    button(6, "Side back", "side-back"),
    button(7, "Side forward", "side-forward"),
];

/// LOD 1 / 2 mm. The 0.7 mm option (value 0x87 = `(7 | 0x80)`, family-c C2.3) has no profile representation.
const LOD: [u8; 2] = [1, 2];

const fn model(id: &'static str, name: &'static str, wired: u16, dongle: u16) -> ModelSpec {
    ModelSpec {
        id,
        name,
        platform: Platform::Xvi,
        // no model code on this platform (events carry none)
        code: 0,
        ids: DeviceIds { vid: VID, wired: Some(wired), dongle: Some(dongle), dongle_4k: None, ble: None },
        dpi: DpiSpec {
            max_units: 840,
            threshold: None,
            stages: 6,
            encoding: "direct",
            // unverified: the doc does not say whether the BE16 on the wire is the DPI or the slider unit
            // (family-e 3.3 "X and Y are BE16", "DPI = value*50"); sent only with X3_ALLOW_UNVERIFIED=1
            verified: false,
        },
        // wired is capped at 1000 Hz on the device (family-e section 4); the spec has one list per model
        polling_rates: &ALL_POLLING_RATES,
        lod_options: &LOD,
        lighting: LightingSpec { modes: &[], visible: false },
        buttons: &BUTTONS,
        // macro event press/release opcodes are unconfirmed (family-e 3.6, section 6)
        macros: false,
        battery: BatterySpec { scale: BatteryScale::Percent, rule: ChargeRule::StateTwoOrThree, over_ble: false },
        read_back: false,
    }
}

pub static MODELS: [ModelSpec; 2] = [model("r6", "R6", 0x0021, 0x0022), model("r5-ultra", "R5 Ultra", 0x0046, 0x0047)];

/// Model of a dongle PID (each XVI dongle belongs to one model, unlike the BK shared receiver).
pub fn by_dongle_pid(pid: u16) -> Option<&'static ModelSpec> {
    MODELS.iter().find(|m| m.ids.dongle == Some(pid))
}
