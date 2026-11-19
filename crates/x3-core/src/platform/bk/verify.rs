//! Read-only check of the family A 0x04 DPI encoding (family-b unknown #1). Nothing is written.
//!
//! The 0x04 read-back returns the stored sensor block. Two writers target this PID and model code.
//! Family A (`X3.exe`, PROTOCOL.md 3.1) stores `enc = u - 1` as lo `[8..15]` / hi `[16..23]`.
//! The R1 page (family-b 3.1) stores a table code in `[8..15]`, a 0/1 half-step flag in `[16..23]`,
//! and sets bit i of `[6]` exactly when stage i has `u > 220`.
//! Only the DPI fields are used as evidence. A `u - 1` block is refuted only by a stage above the
//! model's maximum. A table block is refuted by a code, flag and `[6]` bit that no table row
//! produces, e.g. the factory 26000 DPI stage (hi byte 2). With 520 units (X3, X6) every table block
//! also fits `u - 1`, so the check can confirm `u - 1` but not the table. Blocks that fit both are
//! inconclusive.

use super::codec::table_code;
use crate::model::dpi_from_units;
use crate::platform::{DpiVerdict, ModelSpec};
use crate::profile::DPI_STAGES;
use crate::protocol::checksum16;

/// `DpiSpec::encoding` names of the two candidates.
pub const FORMAT_A: &str = "a";
pub const FORMAT_T: &str = "t";

/// Slider units of a stored `u - 1` stage, if the model can hold it.
fn units_a(lo: u8, hi: u8, max_units: u16) -> Option<u16> {
    let u = u16::from_le_bytes([lo, hi]).checked_add(1)?;
    (u <= max_units).then_some(u)
}

/// Slider units of a stored R1 table stage (any table row, so the table is never refuted too early).
fn units_t(lo: u8, hi: u8, high: bool) -> Option<u16> {
    (0..=350u16).find(|&u| table_code(u) == lo && u8::from((201..=220).contains(&u) || u >= 301) == hi && (u > 220) == high)
}

fn verdict(format: Option<&'static str>, reason: String) -> DpiVerdict {
    DpiVerdict { format, firmware: None, reason: Some(reason) }
}

/// Judge a 0x04 read-back reply (header already checked) for the family A model `spec`.
pub fn judge(spec: &ModelSpec, r: &[u8]) -> DpiVerdict {
    if r.len() < 52 {
        return verdict(None, format!("the 0x04 reply is too short ({} bytes)", r.len()));
    }
    let sum = checksum16(&r[3..50]).to_be_bytes();
    if r[50..52] != sum && r[50..52] != [0, 0] {
        return verdict(None, "the 0x04 reply checksum does not match its data".into());
    }
    let stages: Vec<(usize, u8, u8)> = (0..DPI_STAGES).map(|i| (i, r[8 + i], r[16 + i])).collect();
    let as_a: Vec<Option<u16>> = stages.iter().map(|&(_, lo, hi)| units_a(lo, hi, spec.dpi.max_units)).collect();
    let as_t: Vec<Option<u16>> = stages.iter().map(|&(i, lo, hi)| units_t(lo, hi, r[6] & (1 << i) != 0)).collect();
    let first_bad = |v: &[Option<u16>]| v.iter().position(Option::is_none);
    match (first_bad(&as_a), first_bad(&as_t)) {
        (None, Some(i)) => verdict(
            Some(FORMAT_A),
            format!("stage {} holds {:02X} {:02X}, which no R1 table row produces; it is a valid u-1 value", i + 1, r[8 + i], r[16 + i]),
        ),
        (Some(i), None) => verdict(
            Some(FORMAT_T),
            format!("stage {} holds {:02X} {:02X}, above the model's u-1 range; it is a valid R1 table value", i + 1, r[8 + i], r[16 + i]),
        ),
        (Some(_), Some(_)) => verdict(None, "the stored DPI stages fit neither the u-1 nor the R1 table encoding".into()),
        (None, None) => {
            let enabled = |v: &[Option<u16>], threshold| {
                let dpi = (0..DPI_STAGES).filter(|i| r[5] & (1 << i) != 0).filter_map(|i| v[i]).map(|u| dpi_from_units(threshold, u));
                dpi.map(|d| d.to_string()).collect::<Vec<_>>().join(", ")
            };
            verdict(
                None,
                format!(
                    "every stored DPI stage fits both encodings (u-1 reads {} DPI, the R1 table reads {} DPI)",
                    enabled(&as_a, spec.dpi.threshold),
                    enabled(&as_t, Some(200))
                ),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::bk::{codec, registry};
    use crate::platform::by_id;
    use crate::profile::Profile;

    fn x6() -> &'static ModelSpec {
        by_id("x6").unwrap()
    }

    fn with_sum(mut b: Vec<u8>) -> Vec<u8> {
        let sum = checksum16(&b[3..50]).to_be_bytes();
        b[50..52].copy_from_slice(&sum);
        b
    }

    /// PROTOCOL.md 3.1 default packet, bytes 3..49 (`sub_4155e0` defaults: 800..5000 and 26000 DPI).
    fn factory_default() -> Vec<u8> {
        let hex = "00 00 3F 00 00 0F 1F 2F 3F 63 07 00 00 00 00 00 00 00 02 00 00 02 \
                   FF 00 00 00 FF 00 00 00 FF FF FF 00 00 FF FF FF 00 FF FF 40 00 FF FF FF 01";
        let mut b = vec![0x04, 0x38, 0x01];
        b.extend(hex.split_whitespace().map(|h| u8::from_str_radix(h, 16).unwrap()));
        b.resize(56, 0);
        with_sum(b)
    }

    fn a_block(p: &Profile) -> Vec<u8> {
        codec::build_sensor(x6(), &registry::by_spec(x6()).unwrap().wire, p)
    }

    #[test]
    fn the_family_a_factory_block_settles_u_minus_1() {
        let v = judge(x6(), &factory_default());
        assert_eq!(v.format, Some(FORMAT_A), "{:?}", v.reason);
        assert!(v.reason.unwrap().starts_with("stage 6 holds 07 02"));
        assert_eq!(factory_default(), a_block(&Profile::default()), "our u-1 builder writes the same block");
    }

    #[test]
    fn low_stages_fit_both_encodings_and_stay_inconclusive() {
        let on = [true, true, true, true, true, false, false, false];
        let p = Profile { dpi_units: [16, 32, 48, 64, 100, 0, 0, 0], stage_enabled: on, ..Profile::default() };
        let v = judge(x6(), &a_block(&p));
        assert_eq!(v.format, None);
        assert!(v.reason.unwrap().contains("u-1 reads 800, 1600, 2400, 3200, 5000 DPI, the R1 table reads 650, 1350, 2000, 2700, 4250"));
    }

    #[test]
    fn a_code_no_table_row_produces_settles_u_minus_1() {
        // 400 DPI as u-1 is code 7, which the R1 table skips (350 DPI = 8)
        let p = Profile { dpi_units: [8, 32, 0, 0, 0, 0, 0, 0], ..Profile::default() };
        assert_eq!(judge(x6(), &a_block(&p)).format, Some(FORMAT_A));
        // angle snap on sets [6] bit 0, which the table reserves for stage 1 above 220 units
        let p = Profile { dpi_units: [16, 0, 0, 0, 0, 0, 0, 0], angle_snap: true, ..Profile::default() };
        assert_eq!(judge(x6(), &a_block(&p)).format, Some(FORMAT_A));
    }

    #[test]
    fn a_table_block_also_fits_u_minus_1_so_the_table_is_never_confirmed() {
        let r1 = by_id("r1").unwrap();
        let p = Profile { dpi_units: [16, 32, 48, 64, 200, 210, 250, 0], ..Profile::default() };
        let block = codec::build_sensor(r1, &registry::by_spec(r1).unwrap().wire, &p);
        assert_eq!(judge(x6(), &block).format, None);
    }

    #[test]
    fn a_damaged_or_empty_reply_is_inconclusive() {
        let mut b = factory_default();
        b[50] ^= 0xFF;
        assert!(judge(x6(), &b).reason.unwrap().contains("checksum"));
        assert_eq!(judge(x6(), &[0x04, 0x38]).format, None);
        let zeros = with_sum({
            let mut z = vec![0u8; 56];
            z[..2].copy_from_slice(&[0x04, 0x38]);
            z
        });
        assert_eq!(judge(x6(), &zeros).format, None);
    }
}
