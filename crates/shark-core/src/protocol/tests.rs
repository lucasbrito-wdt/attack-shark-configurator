use super::apply::plan;
use super::events::*;
use super::macros::*;
use super::packets::*;
use super::*;
use crate::profile::*;

fn hex(s: &str) -> Vec<u8> {
    s.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
}

#[test]
fn lighting_defaults_match_spec_example() {
    let expected = hex("05 0F 01 00 03 A8 00 00 FF 01 04 01 AF 00 00");
    assert_eq!(build_lighting(&Profile::default()), expected);
}

#[test]
fn sensor_defaults_match_spec_bytes_and_checksum() {
    let p = build_sensor(&Profile::default());
    let body = hex(
        "00 00 3F 00 00 0F 1F 2F 3F 63 07 00 00 00 00 00 00 00 02 00 00 02 \
         FF 00 00 00 FF 00 00 00 FF FF FF 00 00 FF FF FF 00 FF FF 40 00 FF FF FF 01",
    );
    assert_eq!(p.len(), 56);
    assert_eq!(&p[..3], &[0x04, 0x38, 0x01]);
    assert_eq!(&p[3..50], body.as_slice());
    assert_eq!(checksum16(&p[3..=49]), 0x0E7D);
    assert_eq!(&p[50..52], &[0x0E, 0x7D]);
    assert_eq!(&p[52..], &[0, 0, 0, 0]);
}

#[test]
fn sensor_never_underflows_on_zero_dpi() {
    let prof = Profile { dpi_units: [0; 8], ..Profile::default() };
    let p = build_sensor(&prof);
    assert!(p[8..24].iter().all(|b| *b == 0));
}

#[test]
fn light_brightness_only_for_modes_1_and_9() {
    let mut prof = Profile { brightness: 3, deep_sleep_minutes: 10, light_mode: 1, ..Profile::default() };
    assert_eq!(build_lighting(&prof)[5], 0xA3);
    prof.light_mode = 2;
    assert_eq!(build_lighting(&prof)[5], 0xA8);
}

#[test]
fn polling_has_inverted_check_byte() {
    for (idx, v) in [(0u8, 8u8), (1, 4), (2, 2), (3, 1)] {
        let p = build_polling(&Profile { polling_index: idx, ..Profile::default() });
        assert_eq!(p.len(), 9);
        assert_eq!(&p[..4], &[0x06, 0x09, 0x01, v]);
        assert_eq!(p[4], !v);
    }
}

#[test]
fn button_map_layout_and_checksum() {
    let p = build_buttons(&Profile::default());
    assert_eq!(p.len(), 59);
    assert_eq!(&p[..3], &[0x08, 0x3B, 0x01]);
    // official vendor table: type 0x01 is "Button Off", left/right/middle are 0x02/0x03/0x04
    assert_eq!(&p[3..6], &[0x02, 0x00, 0x00]); // button 0: left click
    assert_eq!(&p[6..9], &[0x03, 0x00, 0x00]); // button 1: right click
    assert_eq!(&p[9..12], &[0x04, 0x00, 0x00]); // button 2: middle
    assert_eq!(&p[12..15], &[0x01, 0x00, 0x00]); // button 3: off
    let sum = checksum16(&p[3..=56]);
    assert_eq!(&p[57..], &[(sum >> 8) as u8, sum as u8]);
}

#[test]
fn button_shortcut_and_macro_encoding() {
    let mut prof = Profile::default();
    prof.buttons[3].action_id = ACTION_SHORTCUT;
    prof.buttons[3].shortcut_mods = 0x05;
    prof.buttons[3].shortcut_key = 0x09;
    prof.buttons[4].action_id = ACTION_MACRO;
    let p = build_buttons(&prof);
    assert_eq!(&p[12..15], &[0x11, 0x05, 0x09]);
    assert_eq!(p[15], 0x12); // macro type
    assert_eq!(p[15 + 2], 5); // macro: b = button index + 1
    assert_eq!(p[15 + 1], 0); // macro: a = 0
}

#[test]
fn resync_literal_bytes() {
    assert_eq!(build_resync(), hex("0C 0A 01 FE 01 FE 00 00 00 00"));
}

fn sample_macro() -> Macro {
    Macro {
        name: "m".into(),
        events: vec![
            MacroEvent { key: 0x04, down: true, delay_ms: 50 },
            MacroEvent { key: 0x04, down: false, delay_ms: 50 },
        ],
        play_mode: PLAY_COUNT,
        loop_count: 2,
    }
}

#[test]
fn macro_packet_layout() {
    let p = build_macro(2, &sample_macro());
    assert_eq!(p.len(), 131);
    assert_eq!(&p[..8], &[0x09, 0x83, 3, 1, 2, 2, 2, 1]);
    assert_eq!(&p[29..33], &[5, 0x04, 5 | 0x80, 0x04]);
    let sum = checksum16(&p[3..=128]);
    assert_eq!(&p[129..], &[(sum >> 8) as u8, sum as u8]);
}

#[test]
fn macro_chunks_wired_and_dongle() {
    let mut pkt = build_macro(0, &sample_macro());
    for (i, b) in pkt.iter_mut().enumerate().skip(3) {
        *b = i as u8; // distinguishable payload
    }
    let wired = chunk_macro(&pkt, false);
    assert_eq!(wired.len(), 3);
    for (idx, c) in wired.iter().enumerate() {
        assert_eq!(c.len(), 64);
        assert_eq!(&c[..4], &[0x09, 0x40, pkt[2], idx as u8]);
    }
    assert_eq!(&wired[0][4..64], &pkt[3..63]);
    assert_eq!(&wired[1][4..64], &pkt[63..123]);
    assert_eq!(&wired[2][4..12], &pkt[123..=130]);
    assert!(wired[2][12..].iter().all(|b| *b == 0));
    let dongle = chunk_macro(&pkt, true);
    assert_eq!(dongle[2][1], 0x0C);
    assert_eq!(dongle[1][1], 0x40);
}

#[test]
fn macro_long_delay_is_split_and_capped() {
    let m = Macro {
        events: vec![MacroEvent { key: 0x05, down: true, delay_ms: 2500 }],
        ..Macro::default()
    };
    let pairs = encode_events(&m.events);
    assert_eq!(pairs.len(), 11);
    assert!(pairs[..10].iter().all(|(_, k)| *k == 0x03));
    assert_eq!(pairs[10].1, 0x05);
    let many = Macro { events: vec![MacroEvent { key: 4, down: true, delay_ms: 10 }; 200], ..Macro::default() };
    assert_eq!(encode_events(&many.events).len(), 50);
    assert_eq!(build_macro(0, &many).len(), 131);
}

#[test]
fn event_pair_vectors_from_spec_4_5() {
    let hb = |awake: bool, battery| Some(Event::Heartbeat { state: u8::from(awake), level: battery });
    assert_eq!(parse_pair(0x4010, 0x0801), hb(true, 8));
    assert_eq!(parse_pair(0x4010, 0x0A01), hb(true, 10));
    assert_eq!(parse_pair(0x4010, 0x0100), hb(false, 1));
    assert_eq!(parse_pair(0x4010, 0x0001), hb(true, 0));
    assert_eq!(parse_pair(0x4010, 0x0B01), hb(true, 11));
    assert_eq!(parse_pair(0x1000, 0x0003), Some(Event::DpiStage(3)));
    assert_eq!(parse_pair(0x6000, 0x0005), Some(Event::Battery(5)));
    assert_eq!(parse_pair(0x0801, 0x1234), None);
    assert_eq!(parse_pair(0x9001, 0), None);
    assert_eq!(parse_pair(0x3000, 0), Some(Event::Unknown { kind: 0x3000, value: 0 }));
}

#[test]
fn event_type_filter_bounds() {
    assert!(type_accepted(0x1000) && type_accepted(0x9000));
    assert!(!type_accepted(0x0FFF) && !type_accepted(0x9001));
}

#[test]
fn event_raw_ble_bytes() {
    assert_eq!(parse_ble(&[0x10, 0x40, 0x01, 0x08]), Some(Event::Heartbeat { state: 1, level: 8 }));
    assert_eq!(parse_ble(&[0x10, 0x40, 0x00, 0x01]), Some(Event::Heartbeat { state: 0, level: 1 }));
    assert_eq!(parse_ble(&[0x00, 0x10, 0x03, 0x00]), Some(Event::DpiStage(3)));
    // swapped order (value first): the type fails the filter, so the swap is used
    assert_eq!(parse_ble(&[0x01, 0x08, 0x10, 0x40]), Some(Event::Heartbeat { state: 1, level: 8 }));
    assert_eq!(parse_ble(&[0x03, 0x00, 0x00, 0x10]), Some(Event::DpiStage(3)));
    // trailing bytes are ignored
    assert_eq!(parse_ble(&[0x10, 0x40, 0x01, 0x08, 0xFF, 0xFF]), Some(Event::Heartbeat { state: 1, level: 8 }));
    assert_eq!(parse_ble(&[0x10, 0x50, 0x01, 0x00]), Some(Event::Ack(1)));
    assert_eq!(parse_ble(&[0x00, 0x70, 0x04, 0x00]), Some(Event::LightMode(4)));
}

#[test]
fn event_raw_usb_bytes() {
    assert_eq!(parse_input_report(&[0x03, 0x10, 0x40, 0x01, 0x08]), Some(Event::Heartbeat { state: 1, level: 8 }));
    assert_eq!(parse_input_report(&[0x03, 0x10, 0x40, 0x00, 0x0A]), Some(Event::Heartbeat { state: 0, level: 10 }));
    assert_eq!(parse_input_report(&[0x03, 0x00, 0x10, 0x03, 0x00]), Some(Event::DpiStage(3)));
    assert_eq!(parse_input_report(&[0x03, 0x00, 0x10, 0x03, 0x00, 0xAA, 0xBB]), Some(Event::DpiStage(3)));
    assert_eq!(parse_input_report(&[0x02, 0x10, 0x40, 0x01, 0x08]), None);
}

#[test]
fn event_parsing_rejects_garbage() {
    assert_eq!(parse_ble(&[]), None);
    assert_eq!(parse_ble(&[1, 2, 3]), None);
    assert_eq!(parse_ble(&[0xFF; 4]), None);
    assert_eq!(parse_input_report(&[0xAA]), None);
    assert_eq!(parse_input_report(&[0x03, 0x10, 0x40, 0x01]), None);
    // no swap and no offset guessing on USB
    assert_eq!(parse_input_report(&[0x03, 0x01, 0x08, 0x10, 0x40]), None);
    assert_eq!(parse_input_report(&[0x10, 0x40, 0x01, 0x08, 0x00]), None);
    assert_eq!(parse_input_report(&[0x05, 0x00, 0x10, 0x02, 0x00]), None);
    // dropped by the type filter
    assert_eq!(parse_input_report(&[0x03, 0x01, 0x08, 0x00, 0x00]), None);
}

#[test]
fn apply_plan_order_and_dirty_groups() {
    let base = Profile::default();
    let all = plan(None, &base, false);
    let ids: Vec<u8> = all.iter().map(|p| p[0]).collect();
    assert_eq!(ids, vec![CMD_SENSOR, CMD_LIGHTING, CMD_POLLING, CMD_BUTTONS]);
    assert!(plan(Some(&base), &base, false).is_empty());
    let mut changed = base.clone();
    changed.polling_index = 1;
    changed.ripple_control = true;
    let ids: Vec<u8> = plan(Some(&base), &changed, false).iter().map(|p| p[0]).collect();
    assert_eq!(ids, vec![CMD_SENSOR, CMD_POLLING]);
}

#[test]
fn apply_plan_sends_macro_only_when_changed() {
    let mut prof = Profile::default();
    prof.buttons[3].action_id = ACTION_MACRO;
    prof.buttons[3].macro_binding = Some(sample_macro());
    let first = plan(None, &prof, false);
    assert_eq!(first.last().map(|p| p[0]), Some(CMD_MACRO));
    assert!(plan(Some(&prof), &prof, false).is_empty());
}

#[test]
fn resync_is_never_emitted_unless_explicitly_requested() {
    let prof = Profile::default();
    // startup, profile switch, import and restore defaults all plan with prev = None
    for prev in [None, Some(&prof)] {
        assert!(plan(prev, &prof, false).iter().all(|p| p[0] != CMD_RESYNC));
    }
    let confirmed = plan(Some(&prof), &prof, true);
    assert_eq!(confirmed[0][0], CMD_RESYNC);
    assert_eq!(confirmed.len(), 5);
}

#[test]
fn observed_ble_type_0x1010_is_the_dpi_stage_of_model_0x10() {
    // FEE4 notifications seen on hardware: type 0x1010 = DPI stage category | model code 0x10 (family-b 4.2)
    assert_eq!(parse_ble(&[0x10, 0x10, 0x02, 0x00]), Some(Event::DpiStage(2)));
    assert_eq!(parse_ble(&[0x10, 0x10, 0x01, 0x00]), Some(Event::DpiStage(1)));
}
