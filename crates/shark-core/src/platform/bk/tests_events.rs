use super::*;
use crate::device::LinkKind::{self, Ble, Dongle, Wired};
use crate::platform::battery::{PowerReading, decode};
use crate::platform::by_id;

fn power(id: &str, link: LinkKind, state: u8, level: u8) -> Option<PowerReading> {
    decode(&by_id(id).unwrap().battery, link, state, level)
}

fn reading(awake: Option<bool>, percent: Option<u8>, charging: Option<bool>) -> Option<PowerReading> {
    Some(PowerReading { awake, percent, charging })
}

#[test]
fn family_b_test_vectors() {
    let usb = |b: &[u8]| BkCodec.parse_report(b);
    assert_eq!(BkCodec.parse_notification(&[0x10, 0x10, 0x02, 0x00]), Some(Event::DpiStage(2)));
    assert_eq!(BkCodec.parse_notification(&[0x10, 0x10, 0x01, 0x00]), Some(Event::DpiStage(1)));
    assert_eq!(usb(&[0x03, 0x10, 0x40, 0x01, 0x07]), Some(Event::Heartbeat { state: 1, level: 7 }));
    assert_eq!(power("r1", Dongle, 1, 7).and_then(|r| r.percent), Some(70));
    assert_eq!(usb(&[0x03, 0x4E, 0x40, 0x00, 0x55]), Some(Event::Heartbeat { state: 0, level: 0x55 }));
    assert_eq!(power("x3-max", Dongle, 0, 0x55), reading(None, Some(85), None));
    assert_eq!(usb(&[0x03, 0xBE, 0x20, 0x42, 0x00]), Some(Event::Polling(0x42)));
    assert_eq!(usb(&[0x03, 0x4E, 0x70, 0x30, 0x00]), Some(Event::LightMode(3)));
    assert_eq!(usb(&[0x03, 0x4E, 0x50, 0x00, 0x00]), Some(Event::Ack(0)));
}

#[test]
fn battery_is_discarded_over_ble_for_the_first_four_pages() {
    for id in ["x6", "x3", "x6-2055", "x3-201b", "r1", "x11"] {
        assert_eq!(power(id, Ble, 1, 8), None, "{id}");
        assert!(power(id, Dongle, 1, 8).is_some(), "{id}");
    }
    assert_eq!(power("x3-max", Ble, 0, 40), reading(None, Some(40), None));
    assert_eq!(power("x3-max", Ble, 2, 40), reading(None, Some(100), Some(false)), "BLE state 2 = 100 %");
}

#[test]
fn family_a_heartbeat_keeps_awake_semantics() {
    assert_eq!(power("x6", Dongle, 1, 8), reading(Some(true), Some(80), None));
    assert_eq!(power("x6", Dongle, 0, 1), reading(Some(false), Some(10), None));
    assert_eq!(power("x6", Dongle, 1, 0), reading(Some(true), None, None), "level 0 keeps the last value");
    assert_eq!(power("x6", Dongle, 1, 11), reading(Some(true), None, None), "level 11 keeps the last value");
}

#[test]
fn family_b_charge_states() {
    assert_eq!(power("x3-max", Dongle, 3, 50), reading(None, Some(50), Some(true)));
    assert_eq!(power("x3-max", Wired, 2, 50), reading(None, Some(100), Some(false)), "wired state 2 = full");
    assert_eq!(power("x3-max", Wired, 1, 50), reading(None, Some(50), None), "states other than 2/3 are undecoded");
    assert_eq!(power("x3-max", Dongle, 0, 101), reading(None, None, None), "percent above 100 is ignored");
}

#[test]
fn c1_battery_events_0x40_and_0x41() {
    assert_eq!(BkCodec.parse_report(&[0x03, 0x16, 0x41, 0x03, 0x50]), Some(Event::Heartbeat { state: 3, level: 80 }));
    assert_eq!(BkCodec.parse_report(&[0x03, 0xA9, 0x40, 0x01, 0x21]), Some(Event::Heartbeat { state: 1, level: 33 }));
    assert_eq!(power("f1", Dongle, 3, 80), reading(None, Some(80), Some(true)));
    assert_eq!(power("f1", Wired, 2, 100), reading(None, Some(100), Some(true)), "C1: charge flag = state 2 or 3");
    assert_eq!(power("x11-pro-v2", Dongle, 1, 33), reading(None, Some(33), Some(false)));
}

#[test]
fn ble_read_back_replies_are_not_events() {
    assert_eq!(BkCodec.parse_notification(&[0xA1, 0x01, 0x02, 0x00]), None);
    assert_eq!(BkCodec.parse_notification(&[0xA0, 0x01, 0x00, 0x00]), None);
}
