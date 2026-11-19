use super::*;

const EN: Locale = Locale::En;

fn view(level: Option<u8>, charging: bool, known: Option<Known>) -> View {
    View { connected: true, link: Some(Link::Dongle), model: Some("X3".into()), level, charging, via_bas: false, known }
}

use crate::tray_glyph::{self, Glyph};

fn mac(v: &View) -> Glyph {
    tray_glyph::render(v, true)
}

fn ink(g: &Glyph) -> f32 {
    g.rgba.chunks(4).map(|p| f32::from(p[3])).sum::<f32>() / 255.0
}

/// Alpha of the pixel in the middle row, `x` columns in.
fn alpha_at(g: &Glyph, x: u32) -> u8 {
    g.rgba[((g.h / 2 * g.w + x) * 4 + 3) as usize]
}

#[test]
fn fill_width_is_proportional_with_a_visible_sliver() {
    assert_eq!(tray_glyph::fill_width_pt(0), 0.0);
    assert!(tray_glyph::fill_width_pt(1) >= 1.5);
    assert!((tray_glyph::fill_width_pt(100) - 2.0 * tray_glyph::fill_width_pt(50)).abs() < 1e-4);
    assert_eq!(tray_glyph::fill_width_pt(250), tray_glyph::fill_width_pt(100));
}

#[test]
fn rendered_fill_grows_with_level_and_sizes_match_platform() {
    let base = ink(&mac(&view(Some(0), false, None)));
    let half = ink(&mac(&view(Some(50), false, None))) - base;
    let full = ink(&mac(&view(Some(100), false, None))) - base;
    assert!((full / half - 2.0).abs() < 0.1, "full={full} half={half}");
    let m = mac(&view(None, false, None));
    assert_eq!((m.w, m.h, m.rgba.len() as u32), (60, 36, 60 * 36 * 4));
    let l = tray_glyph::render(&view(None, false, None), false);
    assert_eq!((l.w, l.h), (30, 22));
}

#[test]
fn alpha_per_state() {
    let fill_x = 7; // inside the fill at 2x, clear of the stroke
    assert_eq!(alpha_at(&mac(&view(Some(100), false, None)), fill_x), 255);
    let known = Known { level: 100, link: Link::Dongle, at: SystemTime::UNIX_EPOCH };
    let stale = alpha_at(&mac(&view(None, false, Some(known))), fill_x);
    assert!((i32::from(stale) - 115).abs() <= 2, "stale={stale}"); // 0.45 * 255
    assert_eq!(alpha_at(&mac(&view(None, false, None)), fill_x), 0, "unknown has no fill");
    assert_eq!(alpha_at(&mac(&view(None, false, None)), 1), 140); // outline at 0.55 (row-centre of the left stroke)
}

#[test]
fn low_is_red_on_linux_only_and_charging_knocks_out_the_fill() {
    let px = |g: &Glyph, x: u32| {
        let i = ((g.h / 2 * g.w + x) * 4) as usize;
        [g.rgba[i], g.rgba[i + 1], g.rgba[i + 2]]
    };
    let linux = tray_glyph::render(&view(Some(10), false, None), false);
    assert_eq!(px(&linux, 4), [255, 69, 58]);
    assert_eq!(px(&mac(&view(Some(10), false, None)), 7), [0, 0, 0]);
    let (plain, bolted) = (mac(&view(Some(100), false, None)), mac(&view(Some(100), true, None)));
    assert!(ink(&bolted) < ink(&plain));
    // at low level there is no fill under the bolt, so it is drawn solid instead
    assert!(ink(&mac(&view(Some(0), true, None))) > ink(&mac(&view(Some(0), false, None))));
}

/// Writes a preview sheet to `$TRAY_PREVIEW_PATH`; a no-op when the variable is unset.
#[test]
fn preview_sheet() {
    let Some(path) = std::env::var_os("TRAY_PREVIEW_PATH") else { return };
    let known = |l| Some(Known { level: l, link: Link::Dongle, at: SystemTime::UNIX_EPOCH });
    let states = [
        view(Some(100), false, None),
        view(Some(80), false, None),
        view(Some(50), false, None),
        view(Some(20), false, None),
        view(Some(5), false, None),
        view(Some(80), true, None),
        view(Some(5), true, None),
        view(None, false, known(80)),
        view(None, false, None),
    ];
    const SCALE: u32 = 3; // nearest-neighbour zoom so single pixels are visible
    let (cell_w, cell_h) = (40 * SCALE * 2, 30 * SCALE * 2);
    let strips: [(bool, [u8; 3], [u8; 3]); 3] = [(true, [0x1E; 3], [255; 3]), (true, [0xEC; 3], [0; 3]), (false, [0x1E; 3], [255; 3])];
    let mut sheet = tiny_skia::Pixmap::new(cell_w * states.len() as u32, cell_h * strips.len() as u32).unwrap();
    for (row, (template, bg, tint)) in strips.iter().enumerate() {
        for (col, v) in states.iter().enumerate() {
            let g = tray_glyph::render(v, *template);
            // linux glyphs are 1x: double them so they share the 2x cell scale
            let z = if *template { SCALE } else { SCALE * 2 };
            let (ox, oy) = (col as u32 * cell_w + 20, row as u32 * cell_h + (cell_h - g.h * z) / 2);
            let data = sheet.data_mut();
            for y in 0..cell_h {
                for x in 0..cell_w {
                    let (gx, gy) = ((x.wrapping_sub(20)) / z, (y.wrapping_sub((cell_h - g.h * z) / 2)) / z);
                    let i = ((row as u32 * cell_h + y) * sheet_w(cell_w, states.len()) + col as u32 * cell_w + x) as usize * 4;
                    let (mut rgb, a) = (bg.map(f32::from), 0.0_f32);
                    let _ = (a, ox, oy);
                    if x >= 20 && gx < g.w && gy < g.h && y >= (cell_h - g.h * z) / 2 {
                        let p = &g.rgba[((gy * g.w + gx) * 4) as usize..][..4];
                        let al = f32::from(p[3]) / 255.0;
                        let src = if *template { tint.map(f32::from) } else { [p[0], p[1], p[2]].map(f32::from) };
                        for k in 0..3 {
                            rgb[k] = src[k] * al + rgb[k] * (1.0 - al);
                        }
                    }
                    data[i..i + 4].copy_from_slice(&[rgb[0] as u8, rgb[1] as u8, rgb[2] as u8, 255]);
                }
            }
        }
    }
    sheet.save_png(path).unwrap();
}

fn sheet_w(cell_w: u32, n: usize) -> u32 {
    cell_w * n as u32
}

#[test]
fn titles() {
    let known = Known { level: 80, link: Link::Dongle, at: SystemTime::UNIX_EPOCH };
    assert_eq!(title(&view(Some(80), false, None), EN), "80%");
    assert_eq!(title(&view(Some(80), true, None), EN), "80% charging");
    assert_eq!(title(&view(Some(10), false, None), EN), "10%!");
    assert_eq!(title(&view(None, false, Some(known)), EN), "~80%");
    assert_eq!(title(&view(None, false, None), EN), "--");
}

#[test]
fn known_tooltip_includes_age_and_ble_note() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(3 * 3600 + 10);
    let known = Known { level: 80, link: Link::Dongle, at: SystemTime::UNIX_EPOCH };
    let mut v = view(None, false, Some(known));
    assert_eq!(sentence(&v, now, EN), "Last reading 80% via 2.4G receiver, 3 h ago.");
    v.link = Some(Link::Ble);
    assert!(sentence(&v, now, EN).ends_with("This mouse does not report battery over Bluetooth."));
}

#[test]
fn disconnected_ignores_stale_live_level() {
    let mut v = view(Some(70), false, None);
    v.connected = false;
    assert_eq!(title(&v, EN), "--");
    assert_eq!(sentence(&v, SystemTime::now(), EN), "Mouse disconnected");
}

#[test]
fn age_buckets() {
    assert_eq!(age_text(Duration::from_secs(30), EN), "just now");
    assert_eq!(age_text(Duration::from_secs(300), EN), "5 min ago");
    assert_eq!(age_text(Duration::from_secs(2 * 86_400), EN), "2 d ago");
}

#[test]
fn rfc3339_roundtrips_with_core_formatter() {
    let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_795_097_000);
    assert_eq!(parse_rfc3339_utc(&shark_core::storage::utc_rfc3339(t)), Some(t));
    assert_eq!(parse_rfc3339_utc("garbage"), None);
    assert_eq!(parse_rfc3339_utc("2026-13-01T00:00:00Z"), None);
}

#[test]
fn pt_br_stale_tooltip_and_unknown_locale_fallback() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(3 * 3600 + 10);
    let known = Known { level: 80, link: Link::Dongle, at: SystemTime::UNIX_EPOCH };
    let v = view(None, false, Some(known));
    assert_eq!(sentence(&v, now, Locale::PtBr), "Última leitura: 80% via receptor 2.4G, há 3 h.");
    assert_eq!(sentence(&v, now, Locale::Es), "Última lectura: 80% vía receptor 2.4G, hace 3 h.");
    assert_eq!(sentence(&v, now, Locale::parse("xx")), sentence(&v, now, EN));
}

#[test]
fn bas_level_is_labelled_without_charge_state() {
    let mut v = view(Some(80), true, None);
    v.via_bas = true;
    assert_eq!(sentence(&v, SystemTime::now(), EN), "Battery 80% (Bluetooth)");
}
