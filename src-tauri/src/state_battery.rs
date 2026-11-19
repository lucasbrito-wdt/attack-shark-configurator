//! Battery honesty (split from `state.rs`): the vendor event wins; over BLE the standard Battery
//! Service (0x2A19) fills in when the vendor level is missing or discarded.

use super::*;
use shark_core::platform::battery::ChargeRule;
use shark_core::storage::{BatteryLink, LastBattery};

impl Inner {
    /// The first four BK pages send a bogus vendor battery over BLE (family-b 4.4).
    fn ble_blind(&self) -> bool {
        self.links.iter().next() == Some(&LinkKind::Ble) && !self.spec.battery.over_ble
    }

    fn vendor_level(&self) -> Option<u8> {
        self.battery_level.filter(|_| !self.ble_blind())
    }

    pub(super) fn battery(&self, mode: LinkMode) -> Battery {
        let blind = self.ble_blind();
        let bas = self.bas_level.filter(|_| mode == LinkMode::Ble);
        let (level, source) = match (self.vendor_level(), bas) {
            (Some(v), _) => (Some(v), BatterySource::VendorEvent),
            (None, Some(b)) => (Some(b), BatterySource::BleBas),
            (None, None) => (None, BatterySource::None),
        };
        Battery {
            level,
            // BAS carries no charge state (101 is not a confirmed charging code)
            charging: if mode == LinkMode::None || blind { None } else { self.live_charging() },
            source,
            reason: (blind && bas.is_none()).then_some("not-reported-over-ble"),
            last_known: self.store.last_battery.get(self.spec.id).cloned(),
        }
    }

    /// A raw 0x2A19 byte: 0..=100 is a level; 101..=255 is not decoded and keeps the previous level.
    pub(super) fn on_bas(&mut self, byte: u8) {
        if byte > 100 {
            log_line(&format!("bas_battery raw {byte} is not a level; keeping {:?}", self.bas_level));
            return;
        }
        self.bas_level = Some(byte);
        if self.vendor_level().is_none() {
            self.remember_battery(LinkKind::Ble, byte);
        }
    }

    /// Family A has no charge state: it charges while the wired PID is present.
    pub(super) fn live_charging(&self) -> Option<bool> {
        if self.spec.battery.rule == ChargeRule::AwakeFlag { Some(self.links.contains(&LinkKind::Wired)) } else { self.charging }
    }

    pub(super) fn remember_battery(&mut self, link: LinkKind, level: u8) {
        let charging = self.live_charging().unwrap_or(false);
        let link = match link {
            LinkKind::Wired => BatteryLink::Usb,
            LinkKind::Dongle => BatteryLink::Dongle,
            LinkKind::Ble => BatteryLink::Ble,
        };
        let at = storage::utc_rfc3339(std::time::SystemTime::now());
        let prev = self.store.last_battery.insert(self.spec.id.to_string(), LastBattery { level, charging, link, at });
        // shortcut: the file is rewritten only when the reading changes, so a restored `at` can lag; add a timed save if that matters
        if prev.is_none_or(|p| (p.level, p.charging, p.link) != (level, charging, link))
            && let Err(e) = self.save()
        {
            eprintln!("could not save settings: {}", e.message);
        }
    }
}
