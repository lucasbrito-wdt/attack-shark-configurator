use super::codec::{self, REPORT_LEN};
use super::registry::{MODELS, VID};
use crate::platform::*;
use crate::profile::{ACTION_MACRO, Profile};

fn spec(id: &str) -> &'static ModelSpec {
    by_id(id).unwrap()
}

fn head(pkt: &[u8], n: usize) -> Vec<u8> {
    pkt[..n].to_vec()
}

#[test]
fn registry_and_detection() {
    assert_eq!(MODELS.len(), 2);
    assert_eq!(detect_usb(VID, 0x0021, None).map(|m| m.id), Some("r6"));
    assert_eq!(detect_usb(VID, 0x0046, None).map(|m| m.id), Some("r5-ultra"));
    assert_eq!(platform_of_vid(VID), Some(Platform::Xvi));
    assert!(self::codec(Platform::Xvi).is_some());
    let x = XviCodec;
    assert_eq!(x.detect_from_report(0x0022, &[4, 1]).map(|m| m.id), Some("r6"));
    assert_eq!(x.detect_from_report(0x0047, &[4, 1]).map(|m| m.id), Some("r5-ultra"));
    assert_eq!(x.detect_from_report(0x0021, &[4, 1]), None);
    assert!(usb_ids().iter().any(|(v, p, _)| *v == VID && *p == 0x0022));
    for m in &MODELS {
        assert!(crate::platform::spec::ALL_POLLING_RATES.starts_with(m.polling_rates));
        assert_eq!(m.dpi_max(), 42_000);
    }
}

use super::XviCodec;
fn codec(p: Platform) -> Option<&'static dyn PlatformCodec> {
    crate::platform::codec(p)
}

#[test]
fn battery_request_matches_doc_vector() {
    // family-e 2.4: 00 00 00 02 02 00 83 00 ...
    let r = codec::battery_request();
    assert_eq!(r.len(), REPORT_LEN);
    assert_eq!(head(&r, 8), [0, 0, 0, 2, 2, 0, 0x83, 0]);
    assert!(r[7..].iter().all(|b| *b == 0));
}

#[test]
fn battery_reply_decodes() {
    // family-e 2.4: 00 A1 00 02 02 00 83 01 4B = charging at 75 percent
    let mut reply = vec![0u8; REPORT_LEN];
    reply[..9].copy_from_slice(&[0, 0xA1, 0, 2, 2, 0, 0x83, 1, 0x4B]);
    let Some(crate::protocol::events::Event::Heartbeat { state, level }) = codec::parse_battery(&reply) else { panic!() };
    let r = crate::platform::battery::decode(&spec("r6").battery, crate::device::LinkKind::Wired, state, level).unwrap();
    assert_eq!((r.percent, r.charging), (Some(75), Some(true)));
    reply[7] = 0;
    let Some(crate::protocol::events::Event::Heartbeat { state, level }) = codec::parse_battery(&reply) else { panic!() };
    let r = crate::platform::battery::decode(&spec("r6").battery, crate::device::LinkKind::Dongle, state, level).unwrap();
    assert_eq!(r.charging, Some(false));
    reply[1] = 0;
    assert_eq!(codec::parse_battery(&reply), None, "not acked");
    reply[1] = 0xA1;
    reply[6] = 0x85;
    assert_eq!(codec::parse_battery(&reply), None, "different command");
}

#[test]
fn dpi_set_layout_and_guard() {
    let p = Profile::default();
    let s = spec("r6");
    assert_eq!(codec::plan(s, None, &p, false, false), Err(PlanError::UnverifiedEncoding));
    let pk = codec::dpi(s, &p);
    // family-e 3.3: [4]=count*4+2 [5]=1 [6]=1 [7]=profile [8]=count; default units 16,32,48,64,100,520 -> 6 stages
    assert_eq!(head(&pk[0], 13), [0, 0, 0, 2, 26, 1, 1, 1, 6, 0x03, 0x20, 0x03, 0x20]);
    assert_eq!(pk[0].len(), REPORT_LEN);
    assert_eq!(head(&pk[1], 9), [0, 0, 0, 2, 2, 1, 2, 1, 1]);
}

#[test]
fn full_plan_order_and_diff() {
    let s = spec("r6");
    let p = Profile::default();
    let full = codec::plan(s, None, &p, false, true).unwrap();
    assert!(full.iter().all(|x| x.len() == REPORT_LEN));
    assert_eq!(head(&full[0], 8), [0, 0, 0, 2, 1, 0, 5, 1]);
    assert_eq!(head(&full[1], 9), [0, 0, 0, 2, 2, 1, 0, 1, 1], "polling index 3 = 1000 Hz = code 1");
    assert!(codec::plan(s, Some(&p), &p, false, false).unwrap().is_empty());
    let mut q = p.clone();
    q.angle_snap = true;
    let diff = codec::plan(s, Some(&p), &q, false, false).unwrap();
    assert_eq!(diff.len(), 5, "only the options group");
    assert_eq!(head(&diff[0], 9), [0, 0, 0, 2, 2, 1, 4, 1, 1]);
    let resync = codec::plan(s, Some(&p), &p, true, true).unwrap();
    assert_eq!(head(&resync[0], 8), [0, 0, 0, 2, 1, 0, 0x0D, 1]);
}

#[test]
fn polling_codes() {
    let mut p = Profile::default();
    let codes: Vec<u8> = (0..7)
        .map(|i| {
            p.polling_index = i;
            codec::polling(&p)[0][8]
        })
        .collect();
    assert_eq!(codes, [8, 4, 2, 1, 32, 64, 128]);
}

#[test]
fn options_sleep_and_lod() {
    let mut p = Profile { sleep_minutes: 15, lift_off: 1, ..Profile::default() };
    let o = codec::options(spec("r5-ultra"), &p);
    assert_eq!(head(&o[3], 10), [0, 0, 0, 2, 3, 1, 7, 1, 0x03, 0x84], "900 s BE");
    p.sleep_minutes = 60;
    assert_eq!(codec::options(spec("r6"), &p)[3][8..10], [0x07, 0x08], "clamped to 1800 s");
    assert_eq!(head(&o[4], 9), [0, 0, 0, 2, 2, 1, 8, 1, 2], "LOD 2 mm");
}

#[test]
fn buttons_layout() {
    let mut p = Profile::default();
    p.buttons[2].action_id = crate::profile::ACTION_SHORTCUT;
    p.buttons[2].shortcut_mods = 1;
    p.buttons[2].shortcut_key = 0x06;
    p.buttons[7].action_id = ACTION_MACRO;
    let b = codec::buttons(&p);
    assert_eq!(b.len(), 4, "macro button is skipped");
    // family-e 3.5: [4]=5+size [5]=3 [6]=0 [7]=profile [8]=buttonID [9]=0 [10]=functionID [11]=size [12..]
    assert_eq!(head(&b[0], 13), [0, 0, 0, 2, 6, 3, 0, 1, 1, 0, 1, 1, 1]);
    assert_eq!(head(&b[2], 14), [0, 0, 0, 2, 7, 3, 0, 1, 3, 0, 4, 2, 1, 6]);
}

#[test]
fn colors_layout() {
    let c = codec::colors(&Profile::default());
    assert_eq!(head(&c[0], 11), [0, 0, 0, 2, 19, 2, 1, 1, 0xFF, 0, 0]);
}
