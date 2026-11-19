use super::codec::*;
use super::frame::*;
use super::registry::by_spec;
use super::*;
use crate::platform::by_id;
use crate::profile::{ACTION_MACRO, ACTION_SHORTCUT, Macro, MacroEvent, Profile};
use crate::protocol::{CMD_LIGHTING, CMD_POLLING, CMD_RESYNC, checksum16, packets};

fn spec(id: &str) -> &'static ModelSpec {
    by_id(id).unwrap()
}

fn wire(id: &str) -> registry::BkWire {
    by_spec(spec(id)).unwrap().wire
}

fn sensor(id: &str, p: &Profile) -> Vec<u8> {
    build_sensor(spec(id), &wire(id), p)
}

fn with_units(units: [u16; 8]) -> Profile {
    Profile { dpi_units: units, stage_enabled: [true; 8], ..Profile::default() }
}

fn assert_sealed(b: &[u8]) {
    assert_eq!((b.len(), &b[..3]), (56, &[0x04, 0x38, 0x01][..]));
    assert_eq!(u16::from_be_bytes([b[50], b[51]]), checksum16(&b[3..=49]));
}

#[test]
fn variant_a_is_family_a_with_the_page_trailer() {
    let p = Profile::default();
    assert_eq!(sensor("x6-2055", &p), packets::build_sensor(&p));
    let x3_pro = sensor("x3-pro", &p);
    assert_eq!((x3_pro[49], &x3_pro[3..49]), (0x03, &packets::build_sensor(&p)[3..49]));
    assert_sealed(&x3_pro);
}

#[test]
fn variant_t_table_codes_match_the_formula_and_dumps() {
    let expected = [(0, 0), (1, 1), (5, 5), (6, 6), (7, 8), (200, 235), (201, 118), (220, 141), (300, 235), (301, 235), (302, 118), (340, 141), (341, 189), (350, 199)];
    for (u, code) in expected {
        assert_eq!(table_code(u), code, "u = {u}");
    }
    let table: Vec<u8> = (301..=340).map(table_code).collect();
    assert_eq!(table[..6], [235, 118, 118, 119, 119, 121]);
    assert_eq!(table_code(999), table_code(350), "clamped to the table end");
}

#[test]
fn variant_t_matches_the_vendor_tables_for_every_slider_value() {
    for (id, table) in [("r1", &super::dpi_tables::R1[..]), ("v3", &super::dpi_tables::V3[..])] {
        for (i, &(dpi, code, flag)) in table.iter().enumerate() {
            let u = i as u16 + 1;
            let b = sensor(id, &with_units([u, 0, 0, 0, 0, 0, 0, 0]));
            assert_eq!((b[8], b[16]), (code, flag), "{id} u = {u}");
            assert_eq!(crate::model::dpi_from_units(Some(200), u), dpi, "{id} u = {u}");
        }
        assert_eq!(usize::from(spec(id).dpi.max_units), table.len(), "{id} slider ends where the table ends");
    }
}

#[test]
fn variant_t_r1_layout() {
    let mut p = with_units([16, 210, 250, 280, 0, 0, 0, 0]);
    (p.angle_snap, p.ripple_control, p.motion_sync, p.active_stage) = (true, false, true, 2);
    let b = sensor("r1", &p);
    assert_eq!((b[3], b[4], b[5]), (1, 0, 0xFF));
    assert_eq!((b[6], b[7]), (0b1100, 0b1100), "R1: [6] = [7] = u > 220");
    assert_eq!(b[8..12], [table_code(16), table_code(210), table_code(250), table_code(280)]);
    assert_eq!(b[16..20], [0, 1, 0, 0], "half step for 201..=220");
    assert_eq!((b[24], &b[25..28], b[49]), (3, &[0xFF, 0, 0][..], 0x02));
    assert_sealed(&b);
}

#[test]
fn variant_t_v3_layout_and_clamp() {
    let mut p = with_units([230, 310, 345, 350, 400, 0, 0, 0]);
    (p.motion_sync, p.angle_snap) = (true, false);
    let b = sensor("x8-se", &p);
    assert_eq!((b[3], b[4]), (1, 0), "V3: [3] motion sync, [4] angle snap");
    assert_eq!(b[6], 0b00011, "221..=340");
    assert_eq!(b[7], 0b11100, "u >= 341 (400 clamps to 350)");
    assert_eq!(b[16..21], [0, 1, 0, 0, 0], "no half step for u >= 341");
    assert_eq!(b[12], table_code(350));
    assert_sealed(&b);
}

#[test]
fn variant_h_encoding_above_and_below_threshold() {
    let mut p = with_units([600, 601, 720, 1, 0, 0, 0, 0]);
    (p.lift_off, p.ripple_control, p.angle_snap, p.motion_sync) = (1, true, false, true);
    let b = sensor("x3-max", &p);
    assert_eq!((b[3], b[4], b[6], b[7]), (0x11, 0x01, 0b110, 0));
    // 600 -> u-1 = 599; 601 -> 30100 DPI -> 300; 720 -> 42000 -> 419
    assert_eq!((b[8], b[16], b[9], b[17], b[10], b[18], b[11]), (0x57, 0x02, 0x2C, 0x01, 0xA3, 0x01, 0));
    assert!(b[25..=49].iter().all(|x| *x == 0), "MS5 sends no colors");
    assert_sealed(&b);
    let x1 = sensor("x1", &with_units([660, 520, 0, 0, 0, 0, 0, 0]));
    assert_eq!((x1[6], u16::from_le_bytes([x1[8], x1[16]]), u16::from_le_bytes([x1[9], x1[17]]), x1[49]), (0b1, 399, 519, 0x02));
}

#[test]
fn direct_encoding_and_flag_layouts() {
    let mut p = with_units([24, 800, 0, 0, 0, 0, 0, 0]);
    (p.lift_off, p.ripple_control, p.angle_snap, p.motion_sync) = (1, true, true, false);
    let f1 = sensor("f1", &p);
    assert_eq!((f1[3], f1[4], f1[6], f1[7]), (0x11, 0x10, 0, 0));
    assert_eq!((f1[8], f1[16], f1[9], f1[17]), (0xB0, 0x04, 0x40, 0x9C), "1200 and 40000 DPI raw");
    let v2 = sensor("x11-pro-v2", &p);
    assert_eq!((v2[3], v2[4], u16::from_le_bytes([v2[9], v2[17]])), (1, 1, 30000), "clamped to 30000");
    assert_sealed(&v2);
}

#[test]
fn lighting_brightness_rule_per_family() {
    let p = |mode| Profile { light_mode: mode, brightness: 3, light_speed: 6, ..Profile::default() };
    let low = |id: &str, mode| build_lighting(&wire(id), &p(mode))[5] & 0x0F;
    assert_eq!((low("x6", 9), low("x6", 5)), (3, 8));
    assert_eq!((low("x6-2055", 5), low("x6-2055", 9), low("x6-2055", 1)), (3, 8, 3));
    assert_eq!(low("x11-pro-v2", 2), 3, "C1 always sends brightness");
    assert_eq!((build_lighting(&wire("x6"), &p(0))[4] & 0x0F, build_lighting(&wire("x11-pro-v2"), &p(0))[4] & 0x0F), (3, 6));
    assert_eq!(build_lighting(&wire("x6"), &Profile::default()), packets::build_lighting(&Profile::default()));
    let b = build_lighting(&wire("f1"), &p(4));
    assert_eq!((b[0], b[3], u16::from_be_bytes([b[11], b[12]])), (CMD_LIGHTING, 0x40, checksum16(&b[3..=10])));
}

#[test]
fn polling_encodings() {
    let v = |enc, idx| polling_value(enc, idx);
    use registry::PollingEncoding::*;
    assert_eq!((0..4).map(|i| v(Interval, i)).collect::<Vec<_>>(), [8, 4, 2, 1]);
    assert_eq!(v(Interval, 6), 1, "legacy pages cap at 1000 Hz");
    assert_eq!((v(DualNibble, 3), v(DualNibble, 1), v(DualNibble, 6)), (0x44, 0x22, 0x74));
    assert_eq!((0..7).map(|i| v(Interval250us, i)).collect::<Vec<_>>(), [0x20, 0x10, 0x08, 0x04, 0x02, 0x01, 0x40]);
    let pkt = build_polling(&wire("f1"), &Profile::default());
    assert_eq!(pkt, [CMD_POLLING, 0x09, 0x01, 0x01, 0xFE, 0, 0, 0, 0], "C1 worked example, 1000 Hz");
    let x3_pro = build_polling(&wire("x3-pro"), &Profile { polling_index: 6, ..Profile::default() });
    assert_eq!((x3_pro[3], x3_pro[4]), (0x40, 0xBF));
}

#[test]
fn framing_4k_checksums() {
    let pkt = [0x06, 0x09, 0x01, 0x01, 0xFE, 0, 0, 0, 0];
    let frames = frame_4k(&pkt, 0x41);
    assert_eq!(frames.len(), 1);
    let f = &frames[0];
    assert_eq!((f.len(), &f[..3], &f[3..12]), (0x41, &[0x04, 0x0E, 0x00][..], &pkt[..]));
    assert_eq!((f[12], f[13]), (0x01, 0x21), "sum of 04 0E 00 06 09 01 01 FE");
    let mut macro_pkt = vec![0x09, 0x6F, 0x01, 0x02];
    macro_pkt.resize(111, 0x11);
    let frames = frame_4k(&macro_pkt, 0);
    assert_eq!(frames.len(), 2);
    assert_eq!((&frames[0][..3], frames[0].len()), (&[0x04, 0x40, 0x00][..], 64));
    assert_eq!(u16::from_be_bytes([frames[0][62], frames[0][63]]), checksum16(&frames[0][..62]));
    assert_eq!((&frames[1][..3], frames[1].len()), (&[0x04, 0x39, 0x01][..], 57));
    assert_eq!(u16::from_be_bytes([frames[1][55], frames[1][56]]), checksum16(&frames[1][..55]));
    assert_eq!(frames[1][3..55], macro_pkt[59..]);
}

#[test]
fn codec_frames_only_the_4k_links() {
    let pkt = [0x06, 0x09, 0x01, 0x01, 0xFE, 0, 0, 0, 0];
    assert_eq!(BkCodec.frame(registry::PID_DONGLE, &pkt), vec![pkt.to_vec()]);
    assert_eq!(BkCodec.frame(PID_DONGLE_4K, &pkt)[0].len(), 0x41);
    assert_eq!(BkCodec.frame(PID_X3_PRO_WIRED, &pkt)[0].len(), 0x40);
    assert!(BkCodec.frame(PID_DONGLE_4K, &[0x09, 0x40, 1, 0]).is_empty(), "no undecoded macro frames");
    assert!(BkCodec.usb_route(PID_DONGLE_4K).output_reports && !BkCodec.usb_route(0xFA60).output_reports);
}

#[test]
fn unverified_dpi_is_refused_unless_allowed() {
    let x6 = spec("x6");
    let p = Profile::default();
    assert_eq!(BkCodec.plan(x6, None, &p, false, false), Err(PlanError::UnverifiedEncoding));
    let allowed = BkCodec.plan(x6, None, &p, false, true).unwrap();
    assert_eq!(allowed[0], packets::build_sensor(&p), "allowed: family A u-1 encoding");
    let polling_only = Profile { polling_index: 1, ..p.clone() };
    assert_eq!(BkCodec.plan(x6, Some(&p), &polling_only, false, false).unwrap().len(), 1);
    assert_eq!(BkCodec.plan(x6, Some(&p), &p, false, false), Ok(Vec::new()));
    assert!(BkCodec.plan(spec("r1"), None, &with_units([16, 200, 0, 0, 0, 0, 0, 0]), false, false).is_ok());
    let high = with_units([200, 201, 0, 0, 0, 0, 0, 0]);
    assert_eq!(BkCodec.plan(spec("r1"), None, &high, false, false), Err(PlanError::UnverifiedEncoding), "T above 10000 DPI");
    assert!(BkCodec.plan(spec("r1"), None, &high, false, true).is_ok());
    assert!(BkCodec.plan(spec("x3-max"), None, &high, false, false).is_ok(), "variant H is decoded");
}

#[test]
fn resync_and_macros_follow_the_model() {
    let mut p = Profile::default();
    p.buttons[3].action_id = ACTION_MACRO;
    p.buttons[3].macro_binding = Some(Macro { events: vec![MacroEvent { key: 4, down: true, delay_ms: 10 }], ..Macro::default() });
    let ids = |id: &str, resync| -> Vec<u8> { BkCodec.plan(spec(id), None, &p, resync, true).unwrap().iter().map(|x| x[0]).collect() };
    assert_eq!(ids("x3-max", true), [CMD_RESYNC, 0x04, 0x05, 0x06, 0x08, 0x09]);
    assert_eq!(ids("x3-pro", false), [0x04, 0x05, 0x06, 0x08], "111-byte macro format is not decoded");
    assert_eq!(ids("f1", true), [0x04, 0x05, 0x06, 0x08], "C1: no 0x0C and no macros");
}

#[test]
fn c1_button_types() {
    let mut p = Profile::default();
    p.buttons[3].action_id = ACTION_SHORTCUT;
    (p.buttons[3].shortcut_mods, p.buttons[3].shortcut_key) = (0x01, 0x06);
    p.buttons[4].action_id = 0x21; // Copy preset
    p.buttons[5].action_id = 0x0A; // Fire Button: no C1 equivalent
    p.buttons[8].action_id = 0x45; // Volume +
    let b = build_button_map(&wire("f1"), &p);
    let slot = |i: usize| &b[3 + 3 * i..6 + 3 * i];
    assert_eq!((slot(0), slot(1), slot(2)), (&[2, 0, 0][..], &[3, 0, 0][..], &[4, 0, 0][..]));
    assert_eq!((slot(3), slot(4), slot(5)), (&[17, 1, 6][..], &[17, 1, 6][..], &[1, 0, 0][..]));
    assert_eq!((slot(6), slot(7), slot(8)), (&[5, 0, 0][..], &[6, 0, 0][..], &[27, 0, 0][..]));
    assert_eq!(u16::from_be_bytes([b[57], b[58]]), checksum16(&b[3..=56]));
    assert_eq!(build_button_map(&wire("x3-max"), &p), packets::build_buttons(&p));
}

#[test]
fn read_back_requests_and_decoding() {
    assert_eq!(read_request(CMD_VERSION, 8), [0xA0, 0x0B, 0x08, 0, 0x01, 0, 0, 0]);
    assert!(read_ready(&[0xA0, 0x01, 0, 0]) && !read_ready(&[0xA0, 0x00]));
    let ver = [0x0B, 0x08, 0x10, 0x05, 129, 129, 129, 129];
    assert_eq!((parse_version(&ver), version_model_hint(&ver)), (Some(0x1005), Some("X6")));
    assert!(split_read(0x1005) && !split_read(0x2001));
    assert_eq!(ble_version(&[0xA1, 0x01, 0x02, 0x00]), Some(0x0102));
    let a = decode_sensor(&packets::build_sensor(&Profile::default())).unwrap();
    assert_eq!((a[0].as_a, a[5].as_a, a[0].enabled, a[6].enabled), (800, 26000, true, false));
    let t = decode_sensor(&sensor("r1", &with_units([16, 210, 250, 0, 0, 0, 0, 0]))).unwrap();
    assert_eq!((t[0].as_t.clone(), t[1].as_t.clone(), t[2].as_t.clone()), (vec![800], vec![11000], vec![15000]));
}
