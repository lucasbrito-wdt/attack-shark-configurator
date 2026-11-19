use super::codec::{build_lighting, build_polling, build_sensor};
use super::frame::CMD_VERSION;
use super::registry::by_spec;
use super::BkCodec;
use crate::platform::{ModelSpec, PlatformCodec, ReadBack, ReadError, by_id};
use crate::profile::Profile;
use crate::protocol::{CMD_LIGHTING, CMD_POLLING, CMD_SENSOR};

/// Replies a device holding `p` would give, built with the apply's own builders.
fn device(spec: &ModelSpec, p: &Profile, version: u16) -> impl FnMut(u8, usize) -> Result<Vec<u8>, ReadError> {
    let wire = by_spec(spec).unwrap().wire;
    let (polling, light, sensor) = (build_polling(&wire, p), build_lighting(&wire, p), build_sensor(spec, &wire, p));
    move |id, _len| match id {
        CMD_VERSION => Ok(vec![CMD_VERSION, 8, (version >> 8) as u8, version as u8, 0, 0, 0, 0]),
        CMD_POLLING => Ok(polling.clone()),
        CMD_LIGHTING => Ok(light.clone()),
        CMD_SENSOR => Ok(sensor.clone()),
        _ => Err(ReadError::Io("unexpected read".into())),
    }
}

fn read(id: &str, p: &Profile) -> Result<ReadBack, ReadError> {
    let spec = by_id(id).unwrap();
    BkCodec.read_back(spec, &mut device(spec, p, 0x1005))
}

fn sample() -> Profile {
    let mut p = Profile {
        lift_off: 1,
        ripple_control: true,
        angle_snap: false,
        motion_sync: true,
        active_stage: 2,
        polling_index: 1,
        debounce: 7,
        sleep_minutes: 12,
        deep_sleep_minutes: 33,
        light_mode: 1,
        brightness: 5,
        light_speed: 6,
        light_color: [1, 2, 3],
        ..Profile::default()
    };
    p.dpi_units = [16, 32, 150, 190, 0, 0, 0, 0];
    p.stage_enabled = [true, true, true, false, false, false, false, false];
    p.stage_colors[2] = [9, 8, 7];
    p
}

#[test]
fn read_back_returns_what_apply_wrote() {
    let p = sample();
    for id in ["x6-2055", "r1", "x8-pro", "x11-pro-v2", "v3"] {
        let r = read(id, &p).unwrap();
        let s = by_id(id).unwrap().capabilities();
        assert_eq!((r.polling_index, r.active_stage, r.debounce), (Some(1), Some(2), Some(7)), "{id}");
        assert_eq!((r.sleep_minutes, r.deep_sleep_minutes, r.light_mode), (Some(12), Some(33), Some(1)), "{id}");
        assert_eq!((r.light_speed, r.light_color), (Some(6), Some([1, 2, 3])), "{id}");
        assert_eq!(r.brightness, s.lighting_modes.get(1).map_or(Some(5), |m| m.uses_brightness.then_some(5)), "{id}");
        assert_eq!(&r.dpi_units[..4], &[Some(16), Some(32), Some(150), Some(190)], "{id}");
        assert_eq!(&r.stage_enabled[2..4], &[Some(true), Some(false)], "{id}");
        assert_eq!(r.stage_colors[2], Some([9, 8, 7]), "{id}");
        // a flag the 0x04 layout does not carry is unknown, never guessed
        let flag = |on: bool, v: bool| if on { Some(v) } else { None };
        assert_eq!(r.ripple_control, flag(s.supports_ripple_control, true), "{id}");
        assert_eq!(r.angle_snap, flag(s.supports_angle_snap, false), "{id}");
        assert_eq!(r.motion_sync, flag(s.supports_motion_sync, true), "{id}");
        assert_eq!(r.lift_off, (!s.lod_options.is_empty()).then_some(1), "{id}");
    }
    assert_eq!(read("r1", &p).unwrap().firmware, Some(0x1005));
    assert_eq!(read("x11-pro-v2", &p).unwrap().firmware, None, "C1 skips the version read");
}

#[test]
fn read_back_refuses_what_it_cannot_decode() {
    let spec = by_id("r1").unwrap();
    let bulk = BkCodec.read_back(spec, &mut device(spec, &Profile::default(), 0x2001));
    assert!(matches!(bulk, Err(ReadError::Decode(m)) if m.contains("bulk")));
    let mut wrong = |id: u8, _: usize| Ok(if id == CMD_VERSION { vec![CMD_VERSION, 8, 0x10, 0, 0, 0, 0, 0] } else { vec![id, 0, 0] });
    assert!(matches!(BkCodec.read_back(spec, &mut wrong), Err(ReadError::Decode(_))));
    let x6 = by_id("x6").unwrap();
    assert!(matches!(BkCodec.read_back(x6, &mut device(x6, &Profile::default(), 0x1005)), Err(ReadError::Unsupported(_))));
}
