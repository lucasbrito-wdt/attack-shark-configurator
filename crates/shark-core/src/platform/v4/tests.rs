use super::codec::*;
use super::registry::*;
use super::*;
use crate::platform::spec::ALL_POLLING_RATES;
use crate::platform::{by_id, detect_usb};

fn m(id: &str) -> &'static V4Model {
    by_spec(by_id(id).unwrap()).unwrap()
}

fn head(b: &[u8]) -> Vec<u8> {
    assert_eq!(b.len(), 64);
    b[..8].to_vec()
}

fn polling(id: &str, index: u8) -> Vec<u8> {
    head(&build_polling(m(id), &Profile { polling_index: index, ..Profile::default() }))
}

#[test]
fn doc_vectors() {
    assert_eq!(polling("x3-pro-v4", 3), [0x08, 0x01, 0, 0, 0, 0, 0, 0xF6]);
    assert_eq!(polling("x3-pro-v4", 6), [0x08, 0x81, 0, 0, 0, 0, 0, 0x76]);
    assert_eq!(polling("r2-v4-legacy", 3), [0x04, 0x00, 0x01, 0, 0, 0, 0, 0xFA]);
    assert_eq!(head(&build_get_id()), [0x8F, 0, 0, 0, 0, 0, 0, 0x70]);
}

#[test]
fn polling_index_is_clamped_to_the_model() {
    assert_eq!(polling("r2-v4", 6)[1], 0x01, "1000 Hz is the wired limit");
}

#[test]
fn device_id_reply() {
    assert_eq!(parse_device_id(&[0x8F, 0xC7, 0x0B, 0, 0, 1]), Some(3015));
    assert_eq!(parse_device_id(&[0x8E, 1, 2, 3, 4]), None);
    assert_eq!(by_device_id(3015).unwrap().id, "x3-pro-v4");
    assert_eq!(by_device_id(1893).unwrap().id, "r2-v4-legacy");
    assert_eq!(by_device_id(0xFFFF_FFFF), None);
}

#[test]
fn dpi_layout() {
    let p = Profile {
        dpi_units: [16, 32, 0, 520, 0, 0, 0, 0],
        stage_enabled: [true, true, true, true, false, false, false, false],
        active_stage: 3,
        stage_colors: [[1, 2, 3]; 8],
        ..Profile::default()
    };
    let b = build_dpi(m("x3-pro-v4"), &p);
    assert_eq!((b[0], b[1], b[2], b[3]), (0x10, 0, 2, 3), "stage 2 is empty and skipped, active repacked");
    assert_eq!(b[7], 0xFF - (0x10 + 2 + 3));
    assert_eq!(b[8..14], [0x20, 0x03, 0x40, 0x06, 0x90, 0x65]);
    assert_eq!(b[24..26], [0x20, 0x03]);
    assert_eq!(b[40..43], [1, 2, 3]);
    assert_eq!(build_dpi(m("r2-v4-legacy"), &p)[0], 0x54);
}

#[test]
fn ch585_settings_and_pan1080_subset() {
    let p = Profile { debounce: 4, angle_snap: true, motion_sync: true, ..Profile::default() };
    let ch = build_all(m("r3-v4"), &p);
    let ids: Vec<u8> = ch.iter().map(|b| b[0]).collect();
    assert_eq!(ids, [0x10, 0x08, 0x04, 0x12, 0x13, 0x14]);
    assert_eq!((ch[2][1], ch[3][1], ch[4][1], ch[5][1]), (4, 1, 0, 1));
    assert_eq!(build_all(m("r3-v4-legacy"), &p).len(), 2);
}

#[test]
fn plan_guards_unverified_dpi_and_diffs() {
    let spec = by_id("x3-pro-v4").unwrap();
    let p = Profile::default();
    assert_eq!(V4Codec.plan(spec, None, &p, false, false), Err(PlanError::UnverifiedEncoding));
    assert_eq!(V4Codec.plan(spec, None, &p, false, true).unwrap().len(), 6);
    assert!(V4Codec.plan(spec, Some(&p), &p, false, false).unwrap().is_empty());
    let q = Profile { debounce: 9, ..p.clone() };
    assert_eq!(V4Codec.plan(spec, Some(&p), &q, false, false).unwrap().len(), 1);
    assert_eq!(V4Codec.plan(spec, Some(&p), &p, true, true).unwrap().len(), 6);
}

#[test]
fn registry_and_detection() {
    for x in &MODELS {
        assert!(ALL_POLLING_RATES.starts_with(x.spec.polling_rates), "{}", x.spec.id);
        assert_eq!(x.spec.ids.vid, VID);
    }
    assert_eq!(detect_usb(VID, 0x4026, None).unwrap().id, "r2-v4-legacy");
    assert_eq!(detect_usb(VID, 0x4026, Some("R3 (v4 legacy)")).unwrap().id, "r3-v4-legacy");
    assert_eq!(detect_usb(VID, 0x402A, None).unwrap().id, "x3-pro-v4-legacy");
    assert_eq!(V4Codec.frame(0x503A, &build_get_id())[0].len(), 65);
}
