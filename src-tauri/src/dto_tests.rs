//! Drift guard: JSON key names must match `ui/src/api/types.ts`.

use super::*;
use serde_json::json;
use shark_core::profile::Macro;

fn keys<T: Serialize>(v: &T) -> Vec<String> {
    let mut k: Vec<String> = serde_json::to_value(v).unwrap().as_object().unwrap().keys().cloned().collect();
    k.sort();
    k
}

fn lib() -> Vec<Macro> {
    vec![Macro { name: "M".into(), events: vec![MacroEvent { key: 4, down: true, delay_ms: 10 }], ..Macro::default() }]
}

#[test]
fn status_and_battery_keys() {
    let status = DeviceStatus {
        connected: true,
        mode: LinkMode::Dongle,
        model: Some("X6 Mouse2".into()),
        awake: true,
        battery: Battery { level: None, charging: None, source: BatterySource::None, reason: Some("not-reported-over-ble"), last_known: None },
    };
    assert_eq!(keys(&status), ["awake", "battery", "connected", "mode", "model"]);
    let v = serde_json::to_value(&status).unwrap();
    assert_eq!(v["battery"], json!({"level": null, "charging": null, "source": "none", "reason": "not-reported-over-ble", "lastKnown": null}));
    let last = shark_core::storage::LastBattery { level: 80, charging: true, link: shark_core::storage::BatteryLink::Usb, at: "2026-11-19T14:03:07Z".into() };
    let b = Battery { level: Some(80), charging: Some(true), source: BatterySource::VendorEvent, reason: None, last_known: Some(last) };
    assert_eq!(
        serde_json::to_value(&b).unwrap(),
        json!({"level": 80, "charging": true, "source": "vendor-event", "reason": null,
               "lastKnown": {"level": 80, "charging": true, "link": "usb", "at": "2026-11-19T14:03:07Z"}})
    );
    assert_eq!(serde_json::to_value(BatterySource::BleBas).unwrap(), "ble-bas");
    assert_eq!(v["mode"], "dongle");
}

#[test]
fn profile_keys() {
    let p = profile_dto(&Profile::default(), 0, &[], &shark_core::platform::default_model().capabilities());
    assert_eq!(
        keys(&p),
        [
            "activeStage", "angleSnap", "buttons", "debounce", "deepSleepMinutes", "dpiStages", "id", "lighting", "lod",
            "motionSync", "name", "polling", "rippleControl", "sleepMinutes"
        ]
    );
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(keys(&p.dpi_stages[0]), ["color", "dpi", "enabled", "index"]);
    assert_eq!(keys(&p.lighting), ["brightness", "color", "mode", "speed"]);
    assert_eq!(keys(&p.buttons[0]), ["action", "button", "label"]);
    assert_eq!(v["buttons"][0]["action"], json!({"kind": "preset", "id": 1}));
    assert_eq!((v["lod"].clone(), v["polling"].clone(), v["dpiStages"][0]["color"].clone()), (json!(1), json!(1000), json!("#FF0000")));
}

#[test]
fn action_ref_variants() {
    let shortcut = serde_json::to_value(ActionRef::Shortcut { key: 4, mods: 1 }).unwrap();
    assert_eq!(shortcut, json!({"kind": "shortcut", "key": 4, "mods": 1}));
    let m = serde_json::to_value(ActionRef::Macro { macro_id: Some(0), name: Some("M".into()) }).unwrap();
    assert_eq!(m, json!({"kind": "macro", "macroId": 0, "name": "M"}));
    let parsed: ActionRef = serde_json::from_value(json!({"kind": "macro", "macroId": 2})).unwrap();
    assert_eq!(parsed, ActionRef::Macro { macro_id: Some(2), name: None });
}

#[test]
fn macro_and_result_keys() {
    let m = macro_dto(&lib()[0], 3);
    assert_eq!(keys(&m), ["events", "id", "loopCount", "loopMode", "name"]);
    assert_eq!(keys(&m.events[0]), ["delayMs", "down", "key"]);
    let input: MacroInput = serde_json::from_value(json!({"name": "x", "events": [], "loopMode": "whileHeld", "loopCount": 1})).unwrap();
    assert_eq!(input.loop_mode, LoopMode::WhileHeld);
    let apply = ApplyResult { status: ApplyStatus::NoDevice, packets: 0, skipped_groups: vec![SkippedGroup { group: "dpi", code: "unverified_encoding" }] };
    assert_eq!(
        serde_json::to_value(&apply).unwrap(),
        json!({"status": "no-device", "packets": 0, "skippedGroups": [{"group": "dpi", "code": "unverified_encoding"}]})
    );
}

#[test]
fn event_payload_keys() {
    let progress =
        ApplyProgress { state: ProgressState::Retry, packets: 2, skipped: 0, failed: 0, attempt: Some(3), reason: None, skipped_groups: Vec::new(), unconfirmed: false };
    assert_eq!(keys(&progress), ["attempt", "failed", "packets", "reason", "skipped", "skippedGroups", "state", "unconfirmed"]);
    assert_eq!(keys(&DpiStageChanged { stage: 1, dpi: 1600 }), ["dpi", "stage"]);
    assert_eq!(keys(&DeviceEventRaw { event: String::new() }), ["event"]);
    let done = DpiVerification { status: VerificationStatus::UnsupportedLink, format: None, reason: Some("ble".into()) };
    assert_eq!(serde_json::to_value(&done).unwrap(), json!({"status": "unsupported_link", "reason": "ble"}));
    let ev = DpiVerificationEvent { state: "done", result: Some(DpiVerification { status: VerificationStatus::Verified, format: Some("a".into()), reason: None }) };
    assert_eq!(serde_json::to_value(&ev).unwrap(), json!({"state": "done", "result": {"status": "verified", "format": "a"}}));
    assert_eq!(serde_json::to_value(DpiVerificationEvent { state: "running", result: None }).unwrap(), json!({"state": "running"}));
    let stored = shark_core::storage::DpiEncoding { verified: true, format: Some("a".into()), at: "2026-11-19T14:03:07Z".into(), firmware: Some(0x1003) };
    assert_eq!(keys(&stored), ["at", "firmware", "format", "verified"]);
}

#[test]
fn catalog_and_snapshot_keys() {
    let c = crate::catalog::action_catalog();
    assert_eq!(keys(&c), ["categories", "keys", "modifiers"]);
    assert_eq!(keys(&c.categories[0]), ["actions", "id", "label"]);
    assert_eq!(keys(&c.keys[0]), ["code", "label"]);
    assert_eq!(keys(&c.modifiers[0]), ["bit", "label"]);
}

#[test]
fn patch_is_partial_and_validated() {
    let caps = shark_core::platform::default_model().capabilities();
    let patch: SettingsPatch = serde_json::from_value(json!({
        "polling": 500, "lod": 2, "dpiStages": [{"index": 1, "dpi": 1600, "color": "#00FF00"}], "lighting": {"mode": 1}
    }))
    .unwrap();
    let mut p = Profile::default();
    apply_patch(&mut p, &patch, &caps).unwrap();
    assert_eq!((p.polling_index, p.lift_off, p.dpi_units[1], p.stage_colors[1], p.light_mode), (2, 1, 32, [0, 255, 0], 1));
    let before = p.clone();
    for bad in [
        json!({"dpi": 1}), json!({"dpi": 26050}), json!({"dpi": 125}), json!({"color": "red"}), json!({"enabled": true, "index": 9}),
    ] {
        let stage: DpiStagePatch = serde_json::from_value(bad).unwrap();
        let patch = SettingsPatch { dpi_stages: Some(vec![DpiStagePatch { index: stage.index, ..stage }]), debounce: Some(3), ..Default::default() };
        assert!(apply_patch(&mut p, &patch, &caps).is_err());
        assert_eq!(p, before, "a failed patch must not change the profile");
    }
    assert!(apply_patch(&mut p, &SettingsPatch { polling: Some(300), ..Default::default() }, &caps).is_err());
}

#[test]
fn dpi_patch_uses_the_model_grid() {
    let caps = shark_core::platform::by_id("x3-max").unwrap().capabilities();
    let stage = |dpi| SettingsPatch { dpi_stages: Some(vec![DpiStagePatch { index: 0, dpi: Some(dpi), enabled: None, color: None }]), ..Default::default() };
    let mut p = Profile::default();
    apply_patch(&mut p, &stage(30_100), &caps).unwrap();
    assert_eq!(p.dpi_units[0], 601);
    apply_patch(&mut p, &stage(42_000), &caps).unwrap();
    assert_eq!(p.dpi_units[0], 720);
    assert_eq!(profile_dto(&p, 0, &[], &caps).dpi_stages[0].dpi, 42_000);
    assert!(apply_patch(&mut p, &stage(30_150), &caps).is_err(), "above 30000 the step is 100");
    assert!(apply_patch(&mut p, &stage(42_100), &caps).is_err());
    assert_eq!(polling_hz(6, &caps), 1000, "4-rate models clamp the index");
}

#[test]
fn set_action_kinds() {
    let mut p = Profile::default();
    set_action(&mut p, 6, &ActionRef::Shortcut { key: 0x06, mods: 1 }, &[]).unwrap();
    assert_eq!((p.buttons[6].action_id, p.buttons[6].shortcut_key), (ACTION_SHORTCUT, 6));
    set_action(&mut p, 2, &ActionRef::Macro { macro_id: Some(0), name: None }, &lib()).unwrap();
    assert_eq!(profile_dto(&p, 0, &lib(), &shark_core::platform::default_model().capabilities()).buttons[2].action, ActionRef::Macro { macro_id: Some(0), name: Some("M".into()) });
    assert!(set_action(&mut p, 2, &ActionRef::Macro { macro_id: Some(9), name: None }, &lib()).is_err());
    assert!(set_action(&mut p, 18, &ActionRef::Preset { id: 1 }, &[]).is_err());
    assert!(set_action(&mut p, 0, &ActionRef::Preset { id: ACTION_MACRO }, &[]).is_err());
}

#[test]
fn settings_follow_the_model() {
    let caps = |id: &str| shark_core::platform::by_id(id).unwrap().capabilities();
    let r1 = profile_dto(&Profile::default(), 0, &[], &caps("r1"));
    assert_eq!((r1.angle_snap, r1.ripple_control, r1.motion_sync, r1.debounce), (Some(false), Some(false), None, Some(4)));
    let r6 = serde_json::to_value(profile_dto(&Profile::default(), 0, &[], &caps("r6"))).unwrap();
    assert_eq!((r6["debounce"].clone(), r6["deepSleepMinutes"].clone(), r6["motionSync"].clone()), (json!(null), json!(null), json!(false)));
    let patch = |v: serde_json::Value| serde_json::from_value::<SettingsPatch>(v).unwrap();
    let mut p = Profile::default();
    for (id, bad) in [
        ("r1", json!({"motionSync": true})),
        ("v3", json!({"rippleControl": true})),
        ("r6", json!({"debounce": 4})),
        ("r6", json!({"sleepMinutes": 31})),
        ("r3-v4-legacy", json!({"angleSnap": true})),
    ] {
        let e = apply_patch(&mut p, &patch(bad.clone()), &caps(id)).unwrap_err();
        assert_eq!(e.code, "invalid_input", "{id} {bad}");
    }
    assert_eq!(p, Profile::default(), "refused patches change nothing");
    apply_patch(&mut p, &patch(json!({"angleSnap": true, "sleepMinutes": 30})), &caps("r6")).unwrap();
    assert!(p.angle_snap && p.sleep_minutes == 30);
}

#[test]
fn read_back_shape() {
    use shark_core::platform::ReadBack;
    let caps = shark_core::platform::by_id("r1").unwrap().capabilities();
    let mut r = ReadBack { polling_index: Some(3), lift_off: Some(0), light_color: Some([0, 0, 255]), ..ReadBack::default() };
    r.dpi_units[0] = Some(250);
    let dto = crate::state::read::read_back_dto(&r, &caps);
    let profile = profile_dto(&Profile::default(), 0, &[], &caps);
    let mut expected = keys(&profile);
    expected.retain(|k| k != "id" && k != "name");
    expected.push("firmware".into());
    expected.sort();
    assert_eq!(keys(&dto), expected, "the profile shape without id and name, plus firmware");
    let v = serde_json::to_value(&dto).unwrap();
    assert_eq!(v["dpiStages"][0], json!({"index": 0, "dpi": 15000, "enabled": null, "color": null}));
    assert_eq!((v["polling"].clone(), v["lod"].clone(), v["buttons"].clone()), (json!(1000), json!(1), json!(null)));
    assert_eq!(v["lighting"], json!({"mode": null, "brightness": null, "speed": null, "color": "#0000FF"}));
}
