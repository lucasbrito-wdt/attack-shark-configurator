use super::bk::registry::{BkDpi, MODELS, PID_DONGLE, PID_DONGLE_4K, PID_SHARED_WIRED, VID, by_receiver_code};
use super::bk::BkCodec;
use super::spec::ALL_POLLING_RATES;
use super::*;
use crate::profile::Profile;

fn id(m: Option<&ModelSpec>) -> Option<&'static str> {
    m.map(|m| m.id)
}

#[test]
fn registry_is_consistent() {
    let ids: std::collections::HashSet<_> = models().map(|m| m.id).collect();
    assert_eq!(ids.len(), models().count(), "ids are unique across platforms");
    for m in &MODELS {
        let s = &m.spec;
        assert!(ALL_POLLING_RATES.starts_with(s.polling_rates), "{}", s.id);
        let expected = match m.wire.dpi {
            BkDpi::A { .. } => "a",
            BkDpi::Table(_) => "t",
            BkDpi::High { .. } => "h",
            BkDpi::Direct { .. } => "direct",
        };
        assert_eq!(s.dpi.encoding, expected, "{}", s.id);
        assert!(s.dpi.threshold.is_none_or(|t| t < s.dpi.max_units), "{}", s.id);
        // BLE PID low byte = model code, except the 0x10 group that shares BLE PID 0x0040
        if let Some((_, pid)) = s.ids.ble
            && s.code != 0x10
        {
            assert_eq!(pid & 0xFF, s.code, "{}", s.id);
        }
    }
    assert_eq!(platform_of_vid(VID), Some(Platform::Bk));
    assert_eq!(platform_of_vid(0x373E), Some(Platform::Xvi));
    assert!(codec(Platform::Bk).is_some() && codec(Platform::Xvi).is_some());
}

#[test]
fn dpi_max_per_model() {
    let max = |i: &str| by_id(i).unwrap().dpi_max();
    assert_eq!(max("x6"), 26_000);
    assert_eq!(max("r1"), 18_000);
    assert_eq!(max("x11"), 22_000);
    assert_eq!(max("x3-max"), 42_000);
    assert_eq!(max("x1"), 40_000);
    assert_eq!(max("v3"), 25_000);
    assert_eq!(max("f1"), 40_000);
    assert_eq!(max("x11-pro-v2"), 30_000);
}

#[test]
fn usb_detection_by_pid_and_name_hint() {
    assert_eq!(id(detect_usb(VID, 0x2055, None)), Some("x6-2055"));
    assert_eq!(id(detect_usb(VID, 0x201C, Some("whatever"))), Some("x3-max"));
    assert_eq!(id(detect_usb(VID, PID_SHARED_WIRED, None)), Some("x6"));
    assert_eq!(id(detect_usb(VID, PID_SHARED_WIRED, Some("X3 Mouse"))), Some("x3"));
    assert_eq!(id(detect_usb(VID, PID_SHARED_WIRED, Some("R1"))), Some("r1"));
    assert_eq!(id(detect_usb(VID, PID_SHARED_WIRED, Some("X3 MAX"))), Some("x6"), "hint must share the PID");
    assert_eq!(detect_usb(VID, PID_DONGLE, None), None, "receiver binds on its first report");
    assert_eq!(detect_usb(0x1234, 0x2055, None), None);
}

#[test]
fn shared_receiver_binds_by_buf1() {
    assert_eq!(id(by_receiver_code(PID_DONGLE, 0x4E)), Some("x3-max"));
    assert_eq!(id(by_receiver_code(PID_DONGLE, 0x10)), Some("x6"), "user's X6 -> R1 page code");
    assert_eq!(id(by_receiver_code(PID_DONGLE, 0x16)), Some("f1"));
    assert_eq!(id(by_receiver_code(PID_DONGLE, 0x77)), Some("v3"), "unknown code -> MS18 like the vendor");
    assert_eq!(id(by_receiver_code(PID_DONGLE_4K, 0xBE)), Some("x11-pro"));
    assert_eq!(id(by_receiver_code(PID_DONGLE_4K, 0x08)), Some("x8-pro"));
    assert_eq!(by_receiver_code(PID_DONGLE_4K, 0x4E), None);
    assert_eq!(by_receiver_code(0x2055, 0x85), None);
    assert_eq!(id(BkCodec.detect_from_report(PID_DONGLE, &[0x03, 0xA9, 0x40, 0x01, 0x21])), Some("x11-pro-v2"));
    assert_eq!(BkCodec.detect_from_report(PID_DONGLE, &[0x01, 0x00]), None);
}

#[test]
fn ble_detection_by_name_then_notification() {
    assert_eq!(id(detect_ble(Some("X3 MAX"), None)), Some("x3-max"));
    assert_eq!(id(detect_ble(Some("X6 Mouse2"), None)), Some("x6"));
    assert_eq!(detect_ble(Some("F1"), None), None, "C1 has no BLE");
    assert_eq!(id(detect_ble(Some("Unknown"), Some(&[0x4E, 0x40, 0x00, 0x55]))), Some("x3-max"));
    assert_eq!(id(detect_ble(None, Some(&[0x10, 0x10, 0x02, 0x00]))), Some("x6"));
    assert_eq!(detect_ble(None, Some(&[0xA1, 0x01, 0x02])), None);
}

#[test]
fn by_name_prefers_the_longest_match() {
    assert_eq!(id(by_name("X11 PRO V2")), Some("x11-pro-v2"));
    assert_eq!(id(by_name("x11 pro")), Some("x11-pro"));
    assert_eq!(id(by_name("X11")), Some("x11"));
    assert_eq!(by_name("X110"), None);
}

#[test]
fn usb_ids_are_unique() {
    let ids = usb_ids();
    for (i, a) in ids.iter().enumerate() {
        assert!(!ids[i + 1..].iter().any(|b| (a.0, a.1) == (b.0, b.1)), "{a:?}");
    }
    assert!(ids.contains(&(VID, PID_DONGLE, LinkKind::Dongle)));
    assert!(ids.contains(&(VID, PID_DONGLE_4K, LinkKind::Dongle)));
    assert!(ids.contains(&(VID, PID_SHARED_WIRED, LinkKind::Wired)));
}

#[test]
fn capabilities_follow_the_spec() {
    let c = default_model().capabilities();
    assert_eq!((c.model_id, c.model_name, c.platform, c.model_code), ("x6", "X6", "bk", 0x10));
    assert!(!c.dpi_verified);
    assert_eq!(c.polling_rates, vec![125, 250, 500, 1000]);
    let c = by_id("x8-pro").unwrap().capabilities();
    assert_eq!(c.polling_rates, ALL_POLLING_RATES.to_vec());
    assert_eq!((c.dpi_high_step_from, c.dpi_max), (Some(26_000), 40_000));
    let c = by_id("f1").unwrap().capabilities();
    assert!(!c.supports_ble && !c.supports_macros && c.lighting_modes.is_empty());
    assert_eq!(c.max_dpi_stages, 6);
}

#[test]
fn codec_hooks_per_platform() {
    let bk = codec(Platform::Bk).unwrap();
    let x6 = by_id("x6").unwrap();
    let r1 = by_id("r1").unwrap();
    let sensor = |spec: &ModelSpec, units: [u16; 8]| {
        let p = crate::profile::Profile { dpi_units: units, ..Default::default() };
        bk.plan(spec, None, &p, false, true).unwrap().into_iter().find(|p| p[0] == crate::protocol::CMD_SENSOR).unwrap()
    };
    assert!(bk.is_unverified_dpi(x6, &sensor(x6, [16, 0, 0, 0, 0, 0, 0, 0])), "family A DPI is unverified");
    assert!(!bk.is_unverified_dpi(r1, &sensor(r1, [200, 16, 0, 0, 0, 0, 0, 0])), "T up to 10000 DPI is decoded");
    assert!(bk.is_unverified_dpi(r1, &sensor(r1, [201, 0, 0, 0, 0, 0, 0, 0])), "T above 10000 DPI is not");
    assert!(!bk.is_unverified_dpi(x6, &[crate::protocol::CMD_POLLING, 9, 1, 8, 0xF7, 0, 0, 0, 0]));

    let xvi = codec(Platform::Xvi).unwrap();
    let r6 = models().find(|m| m.platform == Platform::Xvi).unwrap();
    assert_eq!((xvi.max_polling(r6, LinkKind::Wired), xvi.max_polling(r6, LinkKind::Dongle)), (Some(1000), None));
    assert!(xvi.ack_reply_len().is_some() && xvi.battery_request().is_some());

    let v4 = codec(Platform::V4).unwrap();
    assert!(v4.identify_request(0x5043).is_some() && v4.identify_request(0x402A).is_none(), "only shared PIDs are queried");
    let reply = [&[0x00, 0x8F][..], &3016u32.to_le_bytes()].concat();
    assert_eq!(v4.identify(&reply).map(|m| m.id), Some("r2-v4"));
    assert_eq!(v4.identify(&[0x00, 0x8F, 0xFF, 0xFF, 0xFF, 0xFF]), None, "unknown id is not guessed");
}

/// Every flag in `settings()` matches what the codec really writes: changing the field changes the
/// planned packets exactly when the capability says the model supports it.
#[test]
fn settings_follow_the_codecs() {
    for spec in models() {
        let c = codec(spec.platform).unwrap();
        let s = c.settings(spec);
        let base = Profile::default();
        let writes = |f: &dyn Fn(&mut Profile)| {
            let mut p = base.clone();
            f(&mut p);
            !c.plan(spec, Some(&base), &p, false, true).unwrap().is_empty()
        };
        let id = spec.id;
        assert_eq!(writes(&|p| p.angle_snap = !p.angle_snap), s.angle_snap, "{id} angle snap");
        assert_eq!(writes(&|p| p.ripple_control = !p.ripple_control), s.ripple_control, "{id} ripple");
        assert_eq!(writes(&|p| p.motion_sync = !p.motion_sync), s.motion_sync, "{id} motion sync");
        assert_eq!(writes(&|p| p.debounce += 1), s.debounce.is_some(), "{id} debounce");
        assert_eq!(writes(&|p| p.sleep_minutes += 1), s.sleep_minutes.is_some(), "{id} sleep");
        assert_eq!(writes(&|p| p.deep_sleep_minutes += 1), s.deep_sleep_minutes.is_some(), "{id} deep sleep");
        let modes = spec.capabilities().lighting_modes;
        for mode in 0..modes.len() as u8 {
            let moded = Profile { light_mode: mode, ..base.clone() };
            let brighter = Profile { brightness: moded.brightness % 8 + 1, ..moded.clone() };
            let carries = !c.plan(spec, Some(&moded), &brighter, false, true).unwrap().is_empty();
            assert_eq!(carries, modes[usize::from(mode)].uses_brightness, "{id} brightness in mode {mode}");
        }
    }
    let caps = by_id("r1").unwrap().capabilities();
    assert_eq!((caps.supports_angle_snap, caps.supports_ripple_control, caps.supports_motion_sync), (true, true, false));
    let caps = by_id("r6").unwrap().capabilities();
    assert_eq!((caps.debounce, caps.sleep_minutes.map(|r| r.max)), (None, Some(30)));
    let caps = by_id("r3-v4-legacy").unwrap().capabilities();
    assert!(!caps.supports_angle_snap && caps.debounce.is_none() && caps.sleep_minutes.is_none());
}

#[test]
fn udev_rule_covers_every_usb_id() {
    // Linux users without the rule cannot open the hidraw node of a model the app supports
    let rule = include_str!("../../../../packaging/linux/99-attack-shark.rules").to_lowercase();
    let missing: Vec<String> = usb_ids()
        .iter()
        .map(|(v, p, _)| format!("attrs{{idvendor}}==\"{v:04x}\", attrs{{idproduct}}==\"{p:04x}\""))
        .filter(|line| !rule.contains(line))
        .collect();
    assert!(missing.is_empty(), "add to packaging/linux/99-attack-shark.rules: {missing:#?}");
}
