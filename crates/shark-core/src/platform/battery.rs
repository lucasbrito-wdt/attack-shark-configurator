//! Battery event (`0x40xx` / C1 `0x41xx`) decoding per model and transport.

use crate::device::LinkKind;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatteryScale {
    /// Level 1..=10 = 10..=100 %; anything else keeps the previous value.
    Tenths,
    /// Level 0..=100 = percent; above 100 is ignored.
    Percent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeRule {
    /// Family A: the low byte is the awake flag; charging means "the wired PID is present".
    AwakeFlag,
    /// Family B: state 3 = charging, 2 = full; any other state leaves charging unknown.
    VendorState,
    /// Family C1: state 1 discharging, 2 full, 3 charging; charge flag = state 2 or 3.
    StateTwoOrThree,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatterySpec {
    pub scale: BatteryScale,
    pub rule: ChargeRule,
    /// False when the vendor discards battery events over BLE (BK codes 0x85, 0x4D, 0x10, 0x55).
    pub over_ble: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerReading {
    pub awake: Option<bool>,
    pub percent: Option<u8>,
    pub charging: Option<bool>,
}

/// Decode `state` (value low byte) and `level` (value high byte) received on `link`.
/// `None` when the vendor discards the event on that link.
pub fn decode(spec: &BatterySpec, link: LinkKind, state: u8, level: u8) -> Option<PowerReading> {
    if link == LinkKind::Ble && !spec.over_ble {
        return None;
    }
    let mut percent = match spec.scale {
        BatteryScale::Tenths => (1..=10).contains(&level).then(|| level * 10),
        BatteryScale::Percent => (level <= 100).then_some(level),
    };
    let (awake, charging) = match spec.rule {
        ChargeRule::AwakeFlag => (Some(state != 0), None),
        ChargeRule::VendorState => {
            // wired and BLE show "100%" for state 2; the receiver only knows state 3 (charging)
            if state == 2 && link != LinkKind::Dongle {
                percent = Some(100);
            }
            // only 2 (full) and 3 (charging) are decoded (family-b unknown #8): other states are unknown
            (None, match state {
                2 => Some(false),
                3 => Some(true),
                _ => None,
            })
        }
        ChargeRule::StateTwoOrThree => (None, Some(matches!(state, 2 | 3))),
    };
    Some(PowerReading { awake, percent, charging })
}
