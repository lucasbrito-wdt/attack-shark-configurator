use super::*;
use std::sync::{Arc, Mutex};
use x3_core::device::manager::DeviceMsg;

type Sent = Arc<Mutex<Vec<Vec<Vec<u8>>>>>;

fn inner() -> (Inner, Sent) {
    let sent: Sent = Arc::default();
    let sink = sent.clone();
    let mut i = Inner::new(Store::default(), None, LinkState::default(), Box::new(move |p| sink.lock().unwrap().push(p)));
    // the default X6 has an unverified DPI encoding; apply tests opt in like X3_ALLOW_UNVERIFIED=1
    i.allow_unverified = true;
    (i, sent)
}

/// A link whose model was detected as `id` (USB detection precedes `LinkUp`).
fn connect_as(i: &mut Inner, kind: LinkKind, id: &str) {
    i.on_msg(DeviceMsg::Detected(x3_core::platform::by_id(id).unwrap()));
    i.on_msg(DeviceMsg::LinkUp(kind));
}

fn connect(i: &mut Inner, kind: LinkKind) {
    connect_as(i, kind, "x6");
}

fn hb(link: LinkKind, state: u8, level: u8) -> DeviceMsg {
    DeviceMsg::Event(link, Event::Heartbeat { state, level })
}

#[test]
fn battery_is_null_until_a_vendor_heartbeat() {
    let (mut i, _) = inner();
    connect(&mut i, LinkKind::Dongle);
    let s = i.status();
    assert_eq!((s.battery.level, s.battery.source, s.battery.reason), (None, BatterySource::None, None));
    i.on_msg(hb(LinkKind::Dongle, 1, 0));
    assert_eq!(i.status().battery.level, None);
    i.on_msg(hb(LinkKind::Dongle, 1, 7));
    let b = i.status().battery;
    assert_eq!((b.level, b.source), (Some(70), BatterySource::VendorEvent));
    i.on_msg(hb(LinkKind::Dongle, 1, 11));
    assert_eq!(i.status().battery.level, Some(70));
    i.on_msg(DeviceMsg::LinkDown(LinkKind::Dongle));
    assert_eq!(i.status().battery.level, None);
}

#[test]
fn ble_battery_is_discarded_for_the_first_four_pages() {
    let (mut i, _) = inner();
    connect(&mut i, LinkKind::Ble);
    i.on_msg(hb(LinkKind::Ble, 1, 7));
    let b = i.status().battery;
    assert_eq!((b.level, b.charging, b.source, b.reason), (None, None, BatterySource::None, Some("not-reported-over-ble")));
    assert!(i.status().awake);
    i.on_msg(DeviceMsg::Detected(x3_core::platform::by_id("x3-max").unwrap()));
    i.on_msg(hb(LinkKind::Ble, 0, 40));
    let b = i.status().battery;
    assert_eq!((b.level, b.charging, b.reason), (Some(40), None, None), "state 0 is not decoded");
}

fn bas(byte: u8) -> DeviceMsg {
    DeviceMsg::Event(LinkKind::Ble, Event::BasBattery(byte))
}

#[test]
fn ble_battery_service_fills_in_when_the_vendor_level_is_discarded() {
    let (mut i, _) = inner();
    connect(&mut i, LinkKind::Ble);
    i.on_msg(bas(80));
    let b = i.status().battery;
    assert_eq!((b.level, b.charging, b.source, b.reason), (Some(80), None, BatterySource::BleBas, None));
    let last = b.last_known.expect("BAS updates lastKnown");
    assert_eq!((last.level, last.link), (80, x3_core::storage::BatteryLink::Ble));
    i.on_msg(bas(101));
    let b = i.status().battery;
    assert_eq!((b.level, b.charging, b.source), (Some(80), None, BatterySource::BleBas), "101 is not a level");
    i.on_msg(DeviceMsg::LinkDown(LinkKind::Ble));
    i.on_msg(DeviceMsg::LinkUp(LinkKind::Ble));
    assert_eq!(i.status().battery.reason, Some("not-reported-over-ble"), "BAS is cleared with the BLE link");
}

#[test]
fn vendor_battery_wins_over_the_ble_battery_service() {
    let (mut i, _) = inner();
    connect_as(&mut i, LinkKind::Ble, "x3-max");
    i.on_msg(bas(80));
    i.on_msg(hb(LinkKind::Ble, 3, 40));
    let b = i.status().battery;
    assert_eq!((b.level, b.source), (Some(40), BatterySource::VendorEvent));
    i.on_msg(bas(90));
    let b = i.status().battery;
    assert_eq!((b.level, b.source, b.last_known.map(|l| l.level)), (Some(40), BatterySource::VendorEvent, Some(40)));
}

#[test]
fn last_known_battery_survives_link_drops_and_restarts() {
    let (mut i, _) = inner();
    assert_eq!(i.status().battery.last_known, None);
    connect(&mut i, LinkKind::Dongle);
    i.on_msg(hb(LinkKind::Dongle, 1, 7));
    let last = i.status().battery.last_known.expect("set by the vendor event");
    assert_eq!((last.level, last.charging, last.link), (70, false, x3_core::storage::BatteryLink::Dongle));
    assert!(last.at.ends_with('Z') && last.at.len() == 20, "{}", last.at);
    i.on_msg(DeviceMsg::LinkDown(LinkKind::Dongle));
    connect(&mut i, LinkKind::Ble);
    i.on_msg(hb(LinkKind::Ble, 1, 3));
    let b = i.status().battery;
    assert_eq!((b.level, b.reason), (None, Some("not-reported-over-ble")), "live level unchanged");
    assert_eq!(b.last_known.as_ref(), Some(&last), "a discarded BLE reading does not replace it");
    i.on_msg(DeviceMsg::LinkDown(LinkKind::Ble));
    let reopened = Inner::new(i.store.clone(), None, LinkState::default(), Box::new(|_| ()));
    assert_eq!(reopened.status().battery.last_known, Some(last));
    let json = serde_json::to_value(i.status().battery).unwrap();
    assert_eq!(json["lastKnown"]["link"], "dongle");
}

#[test]
fn charging_follows_the_wired_pid_on_family_a() {
    let (mut i, _) = inner();
    connect(&mut i, LinkKind::Dongle);
    assert_eq!(i.status().battery.charging, Some(false));
    connect(&mut i, LinkKind::Wired);
    let s = i.status();
    assert!(s.battery.charging == Some(true) && s.mode == LinkMode::Wired && s.awake);
    i.on_msg(DeviceMsg::LinkDown(LinkKind::Wired));
    assert_eq!(i.status().battery.charging, Some(false));
}

#[test]
fn detection_switches_capabilities_and_persists_the_model() {
    let (mut i, _) = inner();
    assert_eq!(i.caps.model_id, "x6");
    connect(&mut i, LinkKind::Dongle);
    i.on_msg(DeviceMsg::Detected(x3_core::platform::by_id("f1").unwrap()));
    assert_eq!((i.caps.model_id, i.store.model.as_deref()), ("f1", Some("f1")));
    i.on_msg(hb(LinkKind::Dongle, 3, 80));
    assert_eq!((i.status().battery.level, i.status().battery.charging), (Some(80), Some(true)));
    let reopened = Inner::new(i.store.clone(), None, LinkState::default(), Box::new(|_| ()));
    assert_eq!(reopened.caps.model_id, "f1", "offline capabilities use the last detected model");
}

#[test]
fn unverified_dpi_skips_only_the_dpi_group() {
    use x3_core::protocol::{CMD_POLLING, CMD_SENSOR};
    let dpi_skipped = vec![SkippedGroup { group: "dpi", code: "unverified_encoding" }];
    let (mut i, sent) = inner();
    i.allow_unverified = false;
    connect(&mut i, LinkKind::Wired);
    let first = i.apply(false, true).unwrap();
    assert_eq!((first.status, &first.skipped_groups), (ApplyStatus::Queued, &dpi_skipped), "first full apply: everything but 0x04");
    let ids = |n: usize| sent.lock().unwrap()[n].iter().map(|p| p[0]).collect::<Vec<_>>();
    assert!(!ids(0).contains(&CMD_SENSOR) && ids(0).contains(&CMD_POLLING));
    i.on_msg(DeviceMsg::SendDone { sent: first.packets, skipped: 0, failed: 0 });
    assert!(i.outbox.iter().any(|e| matches!(e, Emit::Progress(p) if p.state == ProgressState::Ok && p.skipped_groups == dpi_skipped)));

    let patch = SettingsPatch {
        polling: Some(250),
        dpi_stages: Some(vec![DpiStagePatch { index: 0, dpi: Some(1000), enabled: None, color: None }]),
        ..Default::default()
    };
    let r = i.update_settings(&patch).unwrap().apply;
    assert_eq!((r.packets, r.skipped_groups), (1, dpi_skipped.clone()), "polling is written, DPI is skipped");
    assert_eq!(ids(1), [CMD_POLLING]);
    i.on_msg(DeviceMsg::SendDone { sent: 1, skipped: 0, failed: 0 });

    let snap = i.snapshot();
    assert!(snap.pending_changes && snap.pending_groups == dpi_skipped, "the skipped DPI is not counted as applied");

    i.allow_unverified = true;
    let allowed = i.apply(false, false).unwrap();
    assert!(allowed.skipped_groups.is_empty());
    assert_eq!(ids(2), [CMD_SENSOR], "the next diff apply sends the pending 0x04 only");
    i.on_msg(DeviceMsg::SendDone { sent: 1, skipped: 0, failed: 0 });
    let snap = i.snapshot();
    assert!(!snap.pending_changes && snap.pending_groups.is_empty());
}

#[test]
fn apply_waits_for_the_model_to_be_identified() {
    let (mut i, sent) = inner();
    i.on_msg(DeviceMsg::LinkUp(LinkKind::Wired));
    assert_eq!(i.apply(false, true).unwrap_err().code, "unsupported_model", "e.g. a V4 shared PID before 0x8F answers");
    i.on_msg(DeviceMsg::Detected(x3_core::platform::by_id("x6").unwrap()));
    let r = i.apply(false, true).unwrap();
    assert_eq!((r.status, sent.lock().unwrap().len()), (ApplyStatus::Queued, 1));
    i.on_msg(DeviceMsg::SendDone { sent: r.packets, skipped: 0, failed: 0 });
    i.on_msg(DeviceMsg::LinkDown(LinkKind::Wired));
    i.on_msg(DeviceMsg::LinkUp(LinkKind::Wired));
    assert_eq!(i.apply(false, true).unwrap_err().code, "unsupported_model", "identification ends with the last link");
}

#[test]
fn polling_rates_follow_the_link_cap() {
    let (mut i, _) = inner();
    let r6 = x3_core::platform::models().find(|m| m.platform == x3_core::platform::Platform::Xvi).unwrap();
    i.on_msg(DeviceMsg::Detected(r6));
    i.on_msg(DeviceMsg::LinkUp(LinkKind::Dongle));
    assert_eq!(i.caps.polling_rates.last(), r6.polling_rates.last());
    i.on_msg(DeviceMsg::LinkUp(LinkKind::Wired));
    assert_eq!(i.caps.polling_rates.last(), Some(&1000), "XVI wired is capped at 1000 Hz");
    let err = i.update_settings(&SettingsPatch { polling: Some(8000), ..Default::default() }).unwrap_err();
    assert_eq!(err.code, "invalid_input");
    i.on_msg(DeviceMsg::LinkDown(LinkKind::Wired));
    assert_eq!(i.caps.polling_rates.last(), r6.polling_rates.last());
}

#[test]
fn status_events_are_deduplicated_and_unknown_events_are_ignored() {
    let (mut i, _) = inner();
    connect(&mut i, LinkKind::Ble);
    i.on_msg(DeviceMsg::DeviceName("X6 Mouse2".into()));
    let n = i.outbox.len();
    i.on_msg(DeviceMsg::Event(LinkKind::Ble, Event::Unknown { kind: 0x9910, value: 2 }));
    assert_eq!(i.outbox.len(), n, "unknown events change nothing");
    assert_eq!(i.status().model.as_deref(), Some("X6 Mouse2"));
}

#[test]
fn dpi_event_updates_profile_and_emits_zero_based_stage() {
    let (mut i, _) = inner();
    i.on_msg(DeviceMsg::Event(LinkKind::Dongle, Event::DpiStage(3)));
    assert_eq!(i.store.current().active_stage, 2);
    assert!(i.outbox.contains(&Emit::DpiStage(DpiStageChanged { stage: 2, dpi: 2400 })));
}

#[test]
fn settings_apply_only_with_a_device_and_track_progress() {
    let (mut i, sent) = inner();
    let patch = SettingsPatch { polling: Some(500), ..Default::default() };
    assert_eq!(i.update_settings(&patch).unwrap().apply.status, ApplyStatus::NoDevice);
    connect(&mut i, LinkKind::Wired);
    let r = i.update_settings(&SettingsPatch { polling: Some(250), ..Default::default() }).unwrap();
    assert_eq!(r.apply.status, ApplyStatus::Queued);
    assert_eq!(sent.lock().unwrap().len(), 1);
    // a second change while sending is deferred, then flushed when the first completes
    assert_eq!(i.update_settings(&SettingsPatch { lod: Some(2), ..Default::default() }).unwrap().apply.packets, 0);
    i.on_msg(DeviceMsg::SendDone { sent: r.apply.packets, skipped: 0, failed: 0 });
    assert_eq!(sent.lock().unwrap().len(), 2);
    i.on_msg(DeviceMsg::SendDone { sent: 1, skipped: 0, failed: 0 });
    assert!(!i.pending_changes());
    assert_eq!(i.apply(false, false).unwrap().status, ApplyStatus::Nothing);
    assert_eq!(i.apply(false, true).unwrap().status, ApplyStatus::Queued);
}

#[test]
fn reset_requires_confirmation_and_sends_resync_only_then() {
    let (mut i, sent) = inner();
    connect(&mut i, LinkKind::Wired);
    assert_eq!(i.reset_profile(0, false).unwrap_err().code, "confirm_required");
    assert!(sent.lock().unwrap().is_empty());
    i.reset_profile(0, true).unwrap();
    assert_eq!(sent.lock().unwrap()[0][0][0], x3_core::protocol::CMD_RESYNC);
}

#[test]
fn profile_list_rules() {
    let (mut i, _) = inner();
    assert_eq!(i.delete_profile(0).unwrap_err().code, "last_profile");
    i.create_profile("  Work ").unwrap();
    assert_eq!((i.store.selected, i.store.current().name.as_str()), (1, "Work"));
    i.delete_profile(0).unwrap();
    assert_eq!(i.store.selected, 0);
    assert!(i.create_profile("   ").is_err());
    assert_eq!(i.select_profile(5).unwrap_err().code, "not_found");
}

#[test]
fn snapshot_keys() {
    let (i, _) = inner();
    let v = serde_json::to_value(i.snapshot()).unwrap();
    let mut keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["capabilities", "dpiEncoding", "macros", "pendingChanges", "pendingGroups", "profile", "profiles", "status"]);
    assert_eq!(v["profiles"][0], serde_json::json!({"id": 0, "name": "Default"}));
}

#[test]
fn read_back_needs_usb_and_holds_applies_until_it_ends() {
    use x3_core::platform::{ReadBack, ReadError};
    let (mut i, sent) = inner();
    connect_as(&mut i, LinkKind::Ble, "r1");
    assert_eq!(i.begin_read().unwrap_err().code, "unsupported", "BLE only");
    connect_as(&mut i, LinkKind::Wired, "x6");
    assert_eq!(i.begin_read().unwrap_err().code, "unsupported", "the X6 (family A) has no read-back");
    connect_as(&mut i, LinkKind::Wired, "r1");
    assert_eq!(i.begin_read().unwrap().id, "r1");
    let held = i.update_settings(&SettingsPatch { polling: Some(250), ..Default::default() }).unwrap().apply;
    assert_eq!((held.status, held.packets, sent.lock().unwrap().len()), (ApplyStatus::Queued, 0, 0), "no write while reading");
    assert_eq!(i.end_read(Err(ReadError::Timeout("0x0B".into()))).unwrap_err().code, "timeout");
    assert_eq!(sent.lock().unwrap().len(), 1, "the held apply runs when the read ends");
    assert_eq!(i.begin_read().unwrap_err().code, "busy");
    i.on_msg(DeviceMsg::SendDone { sent: 1, skipped: 0, failed: 0 });
    i.begin_read().unwrap();
    assert_eq!(i.end_read(Err(ReadError::Decode("bulk".into()))).unwrap_err().code, "decode_failed");
    i.begin_read().unwrap();
    assert_eq!(i.end_read(Ok(ReadBack { polling_index: Some(0), ..ReadBack::default() })).unwrap().polling, Some(125));
}

fn verdict(format: Option<&'static str>) -> Result<x3_core::platform::DpiVerdict, x3_core::platform::ReadError> {
    Ok(x3_core::platform::DpiVerdict { format, firmware: Some(0x1003), reason: Some("evidence".into()) })
}

#[test]
fn dpi_check_runs_once_per_usb_link_until_a_verdict_is_stored() {
    let (mut i, _) = inner();
    let queued = |i: &mut Inner| std::mem::take(&mut i.outbox).iter().filter(|e| matches!(e, Emit::VerifyDpi)).count();
    connect(&mut i, LinkKind::Ble);
    assert_eq!(queued(&mut i), 0, "BLE cannot read the 0x04 block");
    i.on_msg(DeviceMsg::LinkUp(LinkKind::Dongle));
    assert_eq!(queued(&mut i), 1, "the receiver came up for an identified unverified model");
    i.on_msg(DeviceMsg::Event(LinkKind::Dongle, Event::DpiStage(1)));
    assert_eq!(queued(&mut i), 0, "once per connection");
    let spec = i.begin_verify().unwrap().unwrap();
    i.end_verify(spec, verdict(None));
    i.on_msg(DeviceMsg::LinkDown(LinkKind::Dongle));
    i.on_msg(DeviceMsg::LinkUp(LinkKind::Dongle));
    assert_eq!(queued(&mut i), 0, "an inconclusive verdict is stored too; re-running is manual");
    let (mut r1, _) = inner();
    connect_as(&mut r1, LinkKind::Wired, "r1");
    assert_eq!(queued(&mut r1), 0, "verified models are not checked");
    let done = r1.begin_verify().unwrap().unwrap_err();
    assert_eq!((done.status, done.format.as_deref()), (VerificationStatus::Verified, Some("t")));
}

#[test]
fn a_verified_dpi_encoding_enables_dpi_writes() {
    let (mut i, sent) = inner();
    i.allow_unverified = false;
    connect(&mut i, LinkKind::Ble);
    let ble = i.begin_verify().unwrap().unwrap_err();
    assert_eq!(ble.status, VerificationStatus::UnsupportedLink);
    connect(&mut i, LinkKind::Wired);
    assert!(!i.snapshot().capabilities.dpi_verified);
    let spec = i.begin_verify().unwrap().unwrap();
    assert!(i.outbox.iter().any(|e| matches!(e, Emit::DpiVerification(v) if v.state == "running")));
    assert_eq!(i.apply(false, true).unwrap().status, ApplyStatus::Queued, "deferred while reading");
    assert!(sent.lock().unwrap().is_empty());
    let r = i.end_verify(spec, verdict(Some("a")));
    assert_eq!((r.status, r.format.as_deref()), (VerificationStatus::Verified, Some("a")));
    let snap = i.snapshot();
    assert!(snap.capabilities.dpi_verified);
    let stored = snap.dpi_encoding.expect("persisted");
    assert_eq!((stored.verified, stored.format.as_deref(), stored.firmware), (true, Some("a"), Some(0x1003)));
    let first = sent.lock().unwrap()[0].clone();
    assert!(first.iter().any(|p| p[0] == x3_core::protocol::CMD_SENSOR), "the deferred apply includes the DPI");
}

#[test]
fn other_dpi_check_outcomes_keep_dpi_writes_off() {
    let (mut i, _) = inner();
    i.allow_unverified = false;
    connect(&mut i, LinkKind::Wired);
    let spec = i.begin_verify().unwrap().unwrap();
    let r = i.end_verify(spec, Err(x3_core::platform::ReadError::Timeout("not ready".into())));
    assert_eq!(r.status, VerificationStatus::Timeout);
    assert!(i.snapshot().dpi_encoding.is_none(), "link errors are not verdicts");
    let spec = i.begin_verify().unwrap().unwrap();
    let r = i.end_verify(spec, verdict(Some("t")));
    assert_eq!(r.status, VerificationStatus::Verified);
    assert!(r.reason.unwrap().ends_with("X6 writes the a encoding, not t, so DPI writes stay disabled"));
    assert!(!i.snapshot().capabilities.dpi_verified);
    assert_eq!(i.apply(false, true).unwrap().skipped_groups.len(), 1);
}
