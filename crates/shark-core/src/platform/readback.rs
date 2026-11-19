//! Per-model settings the codecs carry, and the device read-back result.
//!
//! Read-back replies are decoded by re-encoding: a field's value is the one candidate whose packet,
//! built by the same builder the apply uses, matches the reply on the bits that field controls.
//! The decoder therefore never knows more than the encoder; anything ambiguous comes back `None`.

use crate::profile::{DPI_STAGES, Profile, Rgb};
use std::fmt;
use std::ops::Range;

/// Which optional settings a model's codec writes. Checked against the codecs in `platform/tests.rs`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SettingsSpec {
    pub angle_snap: bool,
    pub ripple_control: bool,
    pub motion_sync: bool,
    /// Raw profile value range, `None` = not written.
    pub debounce: Option<(u8, u8)>,
    /// Milliseconds per raw debounce unit.
    pub debounce_ms_per_unit: u8,
    pub sleep_minutes: Option<(u8, u8)>,
    pub deep_sleep_minutes: Option<(u8, u8)>,
    /// Lighting modes whose packet carries the brightness; `None` = every mode.
    pub brightness_modes: Option<&'static [u8]>,
}

/// Settings read from the device. `None` = the reply does not carry the field, or it matches no
/// value or several.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReadBack {
    pub firmware: Option<u16>,
    /// Slider units per stage (0 = unused slot).
    pub dpi_units: [Option<u16>; DPI_STAGES],
    pub stage_enabled: [Option<bool>; DPI_STAGES],
    pub stage_colors: [Option<Rgb>; DPI_STAGES],
    pub active_stage: Option<u8>,
    pub polling_index: Option<u8>,
    pub lift_off: Option<u8>,
    pub ripple_control: Option<bool>,
    pub angle_snap: Option<bool>,
    pub motion_sync: Option<bool>,
    pub debounce: Option<u8>,
    pub sleep_minutes: Option<u8>,
    pub deep_sleep_minutes: Option<u8>,
    pub light_mode: Option<u8>,
    pub brightness: Option<u8>,
    pub light_speed: Option<u8>,
    pub light_color: Option<Rgb>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The model or the current link has no read-back.
    Unsupported(String),
    /// The device never reported the data as ready.
    Timeout(String),
    /// The reply has the wrong header or a layout that is not decoded.
    Decode(String),
    Io(String),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ReadError::Unsupported(m) | ReadError::Timeout(m) | ReadError::Decode(m) | ReadError::Io(m)) = self;
        f.write_str(m)
    }
}

/// Reads one feature report: `(report id, length)` to reply bytes.
/// Outcome of the read-only DPI encoding check. `format` is set only when the stored 0x04 block
/// fits exactly one candidate encoding (named as `DpiSpec::encoding`); otherwise `reason` says why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DpiVerdict {
    pub format: Option<&'static str>,
    pub firmware: Option<u16>,
    pub reason: Option<String>,
}

pub type ReadFn<'a> = dyn FnMut(u8, usize) -> Result<Vec<u8>, ReadError> + 'a;

/// The value among `values` whose packet `build(base with set(v))` matches `reply` on the bits of
/// `payload` that change across the candidates. `None` when none or several match, or when the
/// packet does not carry the field (no bit changes).
pub fn invert<V: Copy>(
    build: &dyn Fn(&Profile) -> Vec<u8>,
    base: &Profile,
    set: impl Fn(&mut Profile, V),
    values: impl IntoIterator<Item = V>,
    reply: &[u8],
    payload: Range<usize>,
) -> Option<V> {
    let built: Vec<(V, Vec<u8>)> = values
        .into_iter()
        .map(|v| {
            let mut p = base.clone();
            set(&mut p, v);
            (v, build(&p))
        })
        .collect();
    let first = &built.first()?.1;
    let byte = |b: &[u8], i: usize| b.get(i).copied().unwrap_or(0);
    let mask: Vec<(usize, u8)> =
        payload.map(|i| (i, built.iter().fold(0, |m, (_, b)| m | (byte(b, i) ^ byte(first, i))))).filter(|(_, m)| *m != 0).collect();
    if mask.is_empty() || reply.len() < mask.last().map_or(0, |(i, _)| i + 1) {
        return None;
    }
    let matches = |b: &[u8]| mask.iter().all(|&(i, m)| byte(b, i) & m == byte(reply, i) & m);
    let mut hits = built.iter().filter(|(_, b)| matches(b));
    match (hits.next(), hits.next()) {
        (Some((v, _)), None) => Some(*v),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invert_finds_the_single_match_and_refuses_ambiguity() {
        // byte 3 = debounce, byte 4 = speed clamped at 5
        let build = |p: &Profile| vec![0, 0, 0, p.debounce, p.light_speed.min(5)];
        let base = Profile::default();
        assert_eq!(invert(&build, &base, |p, v| p.debounce = v, 0..=30, &[9, 9, 9, 12, 7], 3..5), Some(12));
        assert_eq!(invert(&build, &base, |p, v| p.light_speed = v, 0..=8, &[0, 0, 0, 0, 5], 3..5), None, "5..=8 all encode as 5");
        assert_eq!(invert(&build, &base, |p, v| p.light_speed = v, 0..=8, &[0, 0, 0, 0, 2], 3..5), Some(2));
        assert_eq!(invert(&build, &base, |p, v| p.motion_sync = v, [false, true], &[0, 0, 0, 0, 0], 3..5), None, "not carried");
        assert_eq!(invert(&build, &base, |p, v| p.debounce = v, 0..=30, &[0, 0, 0], 3..5), None, "short reply");
    }
}
