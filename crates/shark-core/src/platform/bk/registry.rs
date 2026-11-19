//! BK model table (VID 0x1D57): family A (X3/X6, docs/PROTOCOL.md), family B (MS1..MS18,
//! docs/protocol/family-b.md section 1-2) and family C1 (F1, X11 PRO V2, family-c.md C1.1).

use crate::model::ButtonInfo;
use crate::platform::Platform;
use crate::platform::battery::{BatteryScale, BatterySpec, ChargeRule};
use crate::platform::spec::{DeviceIds, DpiSpec, LightingSpec, ModelSpec};

pub const VID: u16 = 0x1D57;
pub const BLE_VID: u16 = 0x045E;
pub const PID_DONGLE: u16 = 0xFA60;
pub const PID_DONGLE_4K: u16 = 0xFA65;
pub const PID_SHARED_WIRED: u16 = 0xFA61;
/// X3 PRO wired goes through `hiddriver_4k_1` (framed output reports).
pub const PID_X3_PRO_WIRED: u16 = 0x211F;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BkFamily {
    /// X3.exe: brightness on modes 1/9, `0x70` raw mode, heartbeat low byte = awake.
    A,
    /// AttackShark.exe: brightness on modes 1/5, `0x70` mode in the high nibble.
    B,
    /// BK-MouseHub (WebHID): raw speed, brightness always, C1 button types, no 0x0C resync.
    C1,
}

/// Bytes `[3]`/`[4]`/`[6]`/`[7]` and the per-stage codes of the variant T sensor packet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableStyle {
    /// R1 page: `[3]` angle snap, `[4]` ripple, `[6]` = `[7]` = `u > 220`.
    R1,
    /// V3 page: `[3]` motion sync, `[4]` angle snap, `[6]` = `221..=340`, `[7]` = `u >= 341`.
    V3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BkDpi {
    /// Family A layout (`enc = u - 1`), `[49]` = trailer (0x01, X3 PRO 0x03).
    A { trailer: u8 },
    /// Variant T: sensor code table (threshold 200).
    Table(TableStyle),
    /// Variant H: `enc = u - 1` up to the threshold, else `DPI/100 - 1`; `colors` = RGB + `[49] = 2`.
    High { colors: bool },
    /// C1 direct: `raw16 = dpi`. `paw3395` selects the `(LOD<<4)|ripple`, `(angle<<4)|motion` flags.
    Direct { paw3395: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PollingEncoding {
    /// Report interval in ms: 8, 4, 2, 1 (125..1000 Hz).
    Interval,
    /// X11 PRO / X8 PRO: `((hs + 1) << 4) | (ls + 1)`.
    DualNibble,
    /// X3 PRO: 250 us units, 8000 Hz = 0x40.
    Interval250us,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BkWire {
    pub family: BkFamily,
    pub dpi: BkDpi,
    pub polling: PollingEncoding,
    /// Factory action id per 0x08 slot, used for the slots the app does not expose (DPI keys, wheel).
    pub factory_buttons: Option<&'static [u8; 18]>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct BkModel {
    pub spec: ModelSpec,
    pub wire: BkWire,
}

const R4: [u16; 4] = [125, 250, 500, 1000];
const R7: [u16; 7] = [125, 250, 500, 1000, 2000, 4000, 8000];
const LOD: [u8; 2] = [1, 2];

pub const A_LIGHT_MODES: [&str; 12] = [
    "LED Off", "Static", "Breathing", "Neon", "Color Breathing", "Color Static", "Mixed Breathing", "Rainbow Wave",
    "Lightning", "Static Mixed Color", "Marquee", "Marquee 2",
];
const C1_LIGHT_MODES: [&str; 8] = ["Static", "Breathing", "Mono Cycle", "Rainbow", "Marquee", "Color Flow", "APM", "Off"];

const fn button(id: u8, label: &'static str, position: &'static str) -> ButtonInfo {
    ButtonInfo { id, label, position }
}

/// Buttons 1, 2, 3, 7, 8 of the X3 page (family B pages keep the same 18 slots).
const LEGACY_BUTTONS: [ButtonInfo; 5] = [
    button(0, "Left button", "left"),
    button(1, "Right button", "right"),
    button(2, "Middle button", "middle"),
    button(6, "Side back", "side-back"),
    button(7, "Side forward", "side-forward"),
];
/// C1.5 default map: 0..2 = L, R, M; 4 = back; 5 = forward.
const C1_BUTTONS: [ButtonInfo; 5] = [
    button(0, "Left button", "left"),
    button(1, "Right button", "right"),
    button(2, "Middle button", "middle"),
    button(4, "Side back", "side-back"),
    button(5, "Side forward", "side-forward"),
];

/// Factory button defaults (`sub_4155e0`): X6.exe @0x41D9D0 and X3.exe @0x41C9E0. Observed; the X6
/// table, encoded with the official action table, restored an X6 whose DPI keys and wheel were off.
const X6_FACTORY_BUTTONS: [u8; 18] = [0x01, 0x02, 0x03, 0x06, 0x07, 0x08, 0x04, 0x05, 0x34, 0, 0, 0, 0, 0, 0, 0, 0x0C, 0x0B];
const X3_FACTORY_BUTTONS: [u8; 18] = [0x01, 0x02, 0x03, 0x06, 0x34, 0x08, 0x04, 0x05, 0x34, 0, 0, 0, 0, 0, 0, 0, 0x0C, 0x0B];

const fn encoding_name(dpi: BkDpi) -> &'static str {
    match dpi {
        BkDpi::A { .. } => "a",
        BkDpi::Table(_) => "t",
        BkDpi::High { .. } => "h",
        BkDpi::Direct { .. } => "direct",
    }
}

/// Family B page with its wired and BLE PID on the shared FA60 receiver.
const fn page(id: &'static str, name: &'static str, code: u8, wired: u16, ble: u16, dpi: BkDpi) -> BkModel {
    let tenths = matches!(code, 0x85 | 0x4D | 0x10);
    BkModel {
        spec: ModelSpec {
            id,
            name,
            platform: Platform::Bk,
            code: code as u16,
            ids: DeviceIds { vid: VID, wired: Some(wired), dongle: Some(PID_DONGLE), dongle_4k: None, ble: Some((BLE_VID, ble)) },
            dpi: DpiSpec { max_units: 520, threshold: None, stages: 8, encoding: encoding_name(dpi), verified: true },
            polling_rates: &R4,
            lod_options: &[],
            lighting: LightingSpec { modes: &A_LIGHT_MODES, visible: false },
            buttons: &LEGACY_BUTTONS,
            macros: true,
            battery: BatterySpec {
                scale: if tenths { BatteryScale::Tenths } else { BatteryScale::Percent },
                rule: ChargeRule::VendorState,
                over_ble: !matches!(code, 0x85 | 0x4D | 0x10 | 0x55),
            },
            read_back: true,
        },
        wire: BkWire { family: BkFamily::B, dpi, polling: PollingEncoding::Interval, factory_buttons: None },
    }
}

impl BkModel {
    const fn units(mut self, max_units: u16, threshold: Option<u16>) -> Self {
        self.spec.dpi.max_units = max_units;
        self.spec.dpi.threshold = threshold;
        self
    }

    const fn lod(mut self) -> Self {
        self.spec.lod_options = &LOD;
        self
    }

    const fn lit(mut self) -> Self {
        self.spec.lighting.visible = true;
        self
    }

    const fn stages(mut self, n: u8) -> Self {
        self.spec.dpi.stages = n;
        self
    }

    const fn no_macros(mut self) -> Self {
        self.spec.macros = false;
        self
    }

    /// 4K receiver (FA65) with 125..8000 Hz.
    const fn k4(mut self, polling: PollingEncoding) -> Self {
        self.spec.ids.dongle_4k = Some(PID_DONGLE_4K);
        self.spec.polling_rates = &R7;
        self.wire.polling = polling;
        self
    }

    /// Family A device (X3.exe): shares FA61/FA60/BLE 0040 and model code 0x10 with the R1 page.
    /// Family A sends `enc = u - 1`, the R1 page sends variant T for the same PID (family-b unknown #1):
    /// the DPI write is unverified until a read-back settles it.
    const fn family_a(id: &'static str, name: &'static str, factory: &'static [u8; 18]) -> Self {
        let mut m = page(id, name, 0x10, PID_SHARED_WIRED, 0x0040, BkDpi::A { trailer: 0x01 }).lod();
        m.spec.dpi.verified = false;
        m.spec.battery.rule = ChargeRule::AwakeFlag;
        m.spec.read_back = false;
        m.wire.family = BkFamily::A;
        m.wire.factory_buttons = Some(factory);
        m
    }

    /// Family C1 (BK-MouseHub): wired + FA60 only, direct DPI, percent battery.
    const fn c1(id: &'static str, name: &'static str, code: u8, wired: u16, dpi: BkDpi) -> Self {
        let mut m = page(id, name, code, wired, 0, dpi).stages(6).no_macros();
        m.spec.ids.ble = None;
        m.spec.lighting.modes = &[];
        m.spec.buttons = &C1_BUTTONS;
        m.spec.battery = BatterySpec { scale: BatteryScale::Percent, rule: ChargeRule::StateTwoOrThree, over_ble: false };
        m.wire.family = BkFamily::C1;
        m
    }

    const fn c1_lighting(mut self) -> Self {
        self.spec.lighting = LightingSpec { modes: &C1_LIGHT_MODES, visible: true };
        self
    }
}

const fn t(style: TableStyle) -> BkDpi {
    BkDpi::Table(style)
}

const fn h(colors: bool) -> BkDpi {
    BkDpi::High { colors }
}

use BkDpi::A;
use PollingEncoding::{DualNibble, Interval250us};
use TableStyle::{R1, V3};

/// MS7+ pages (X11 PRO onwards) send macros with the undecoded 111-byte 0x09 encoder (family-b 3.5,
/// unknown #6): macros stay unsupported there.
/// Order matters: the first match wins for shared PIDs and model codes (X6 before X3 before R1).
// inferred: T style per page; only R1 (MS3) and V3 (MS18) were decompiled. MS4 follows R1 (same
// generation, max <= 340); every other T page follows V3 because it reaches u >= 341 or shares MS18's code path.
pub static MODELS: [BkModel; 22] = [
    BkModel::family_a("x6", "X6", &X6_FACTORY_BUTTONS),
    BkModel::family_a("x3", "X3", &X3_FACTORY_BUTTONS),
    page("x6-2055", "X6", 0x85, 0x2055, 0x0085, A { trailer: 0x01 }).lod().lit(),
    page("x3-201b", "X3", 0x4D, 0x201B, 0x004D, A { trailer: 0x01 }).lod(),
    page("r1", "R1", 0x10, PID_SHARED_WIRED, 0x0040, t(R1)).units(280, Some(200)).lit(),
    page("x11", "X11", 0x55, 0xFA55, 0x0055, t(R1)).units(320, Some(200)).lit(),
    page("x3-max", "X3 MAX", 0x4E, 0x201C, 0x004E, h(false)).units(720, Some(600)).lod(),
    page("x1", "X1", 0xB1, 0x2111, 0x00B1, h(true)).units(660, Some(520)).lod(),
    page("x11-pro", "X11 PRO", 0xBE, 0x211D, 0x00BE, t(V3)).units(340, Some(200)).k4(DualNibble).no_macros(),
    page("x3-pro", "X3 PRO", 0x01, PID_X3_PRO_WIRED, 0x0001, A { trailer: 0x03 }).lod().k4(Interval250us).no_macros(),
    page("v5-se", "V5 SE", 0x03, 0x2121, 0x0003, t(V3)).units(340, Some(200)).no_macros(),
    page("x11-se", "X11 SE", 0x07, 0x2125, 0x0007, t(V3)).units(340, Some(200)).no_macros(),
    page("x8-se", "X8 SE", 0x02, 0x2120, 0x0002, t(V3)).units(350, Some(200)).no_macros(),
    page("x8-plus", "X8 Plus", 0x06, 0x2124, 0x0006, h(true)).units(660, Some(520)).lod().no_macros(),
    page("x8-pro", "X8 PRO", 0x08, 0x2126, 0x0008, h(true)).units(660, Some(520)).lod().k4(DualNibble).no_macros(),
    page("g3", "G3", 0x04, 0x2122, 0x0004, t(V3)).units(350, Some(200)).no_macros(),
    page("g3-pro", "G3 PRO", 0x0E, 0x212C, 0x000E, t(V3)).units(350, Some(200)).no_macros(),
    page("v3-pro", "V3 PRO", 0xE5, 0x2155, 0x00E5, t(V3)).units(350, Some(200)).no_macros(),
    page("v6", "V6", 0x24, 0x2224, 0x0224, t(V3)).units(350, Some(200)).no_macros(),
    // MS18 shows 6 stages
    page("v3", "V3", 0xEA, 0x215A, 0x00EA, t(V3)).units(350, Some(200)).stages(6).no_macros(),
    BkModel::c1("f1", "F1", 0x16, 0x2416, BkDpi::Direct { paw3395: true }).units(800, None).lod(),
    BkModel::c1("x11-pro-v2", "X11 PRO V2", 0xA9, 0x22A9, BkDpi::Direct { paw3395: false }).units(600, None).c1_lighting(),
];

/// Page the vendor binds unknown codes to on the FA60 receiver ("anything else -> MS18").
const FALLBACK_ID: &str = "v3";

pub fn by_spec(spec: &ModelSpec) -> Option<&'static BkModel> {
    MODELS.iter().find(|m| m.spec.id == spec.id)
}

/// Shared receiver rule (family-b 1.2): the page is chosen by `buf[1]` of the first input report.
/// FA60 maps every code, unknown ones to V3 like the vendor; FA65 accepts only X11 PRO, X3 PRO, X8 PRO.
pub fn by_receiver_code(pid: u16, code: u8) -> Option<&'static ModelSpec> {
    let code = u16::from(code);
    match pid {
        PID_DONGLE => MODELS
            .iter()
            .find(|m| m.spec.ids.dongle == Some(pid) && m.spec.code == code)
            .or_else(|| MODELS.iter().find(|m| m.spec.id == FALLBACK_ID))
            .map(|m| &m.spec),
        PID_DONGLE_4K => MODELS.iter().find(|m| m.spec.ids.dongle_4k == Some(pid) && m.spec.code == code).map(|m| &m.spec),
        _ => None,
    }
}

/// BLE: the low byte of the first notification's type is the model code. Read-back replies
/// (`A0`/`A1`/`A2`/`A9`) are skipped.
pub fn by_ble_notification(data: &[u8]) -> Option<&'static ModelSpec> {
    let code = u16::from(*data.first()?);
    if matches!(code, 0xA0 | 0xA1 | 0xA2 | 0xA9) {
        return None;
    }
    MODELS.iter().find(|m| m.spec.ids.ble.is_some() && m.spec.code == code).map(|m| &m.spec)
}
