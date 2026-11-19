//! Anti-aliased battery glyph for the tray, drawn with tiny-skia. Geometry is in points and scaled
//! to pixels: 2x for the macOS menu bar (template, black), 1x for the Linux AppIndicator (white).

use crate::tray::{LOW_LEVEL, View};
use tiny_skia::{FillRule, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

// Canvas and body, in points. The canvas keeps ~2.5 pt right padding so the macOS title never touches the nub.
const CANVAS_W: f32 = 30.0;
const BODY_W: f32 = 25.0;
const BODY_H: f32 = 12.0;
const STROKE: f32 = 1.5;
const BODY_RADIUS: f32 = 3.5;
const NUB_GAP: f32 = 1.0;
const NUB_W: f32 = 1.5;
const NUB_H: f32 = 4.0;
const FILL_INSET: f32 = 2.5;
const FILL_RADIUS: f32 = 1.25;
const MIN_SLIVER: f32 = 1.5;

const NUB_ALPHA: f32 = 0.4;
const STALE_ALPHA: f32 = 0.45;
const UNKNOWN_ALPHA: f32 = 0.55;

pub struct Glyph {
    /// Straight (non-premultiplied) RGBA.
    pub rgba: Vec<u8>,
    pub w: u32,
    pub h: u32,
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let k = r * 0.552_284_8; // cubic approximation of a quarter circle
    let mut p = PathBuilder::new();
    p.move_to(x + r, y);
    p.line_to(x + w - r, y);
    p.cubic_to(x + w - r + k, y, x + w, y + r - k, x + w, y + r);
    p.line_to(x + w, y + h - r);
    p.cubic_to(x + w, y + h - r + k, x + w - r + k, y + h, x + w - r, y + h);
    p.line_to(x + r, y + h);
    p.cubic_to(x + r - k, y + h, x, y + h - r + k, x, y + h - r);
    p.line_to(x, y + r);
    p.cubic_to(x, y + r - k, x + r - k, y, x + r, y);
    p.close();
    p.finish()
}

fn bolt(cx: f32, cy: f32, s: f32) -> Option<Path> {
    let pts = [(1.2, -4.2), (-2.6, 0.6), (-0.4, 0.6), (-1.2, 4.2), (2.6, -0.6), (0.4, -0.6)];
    let mut p = PathBuilder::new();
    for (i, (x, y)) in pts.iter().enumerate() {
        let (x, y) = (cx + x * s, cy + y * s);
        if i == 0 { p.move_to(x, y) } else { p.line_to(x, y) }
    }
    p.close();
    p.finish()
}

/// Alpha coverage (0..=1) of a path drawn on an empty canvas.
fn coverage(w: u32, h: u32, draw: impl FnOnce(&mut Pixmap, &Paint)) -> Vec<f32> {
    let mut pm = Pixmap::new(w, h).expect("non-zero glyph canvas");
    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    paint.anti_alias = true;
    draw(&mut pm, &paint);
    pm.pixels().iter().map(|p| f32::from(p.alpha()) / 255.0).collect()
}

/// Fill width in points for `level` percent: proportional, with a minimum visible sliver above 0.
pub fn fill_width_pt(level: u8) -> f32 {
    let inner = BODY_W - 2.0 * FILL_INSET;
    if level == 0 { 0.0 } else { (inner * f32::from(level.min(100)) / 100.0).max(MIN_SLIVER) }
}

/// Render the glyph. `template` = macOS menu bar (2x, black); otherwise Linux (1x, white / red when low).
pub fn render(v: &View, template: bool) -> Glyph {
    let (level, stale) = match (v.live(), v.known) {
        (Some(l), _) => (Some(l), false),
        (None, Some(k)) => (Some(k.level), true),
        _ => (None, false),
    };
    let low = level.is_some_and(|l| l <= LOW_LEVEL);
    let (s, h_pt) = if template { (2.0_f32, 18.0_f32) } else { (1.0, 22.0) };
    let (w, h) = ((CANVAS_W * s) as u32, (h_pt * s) as u32);
    let (base, fill_rgb): ([f32; 3], [f32; 3]) = match (template, low) {
        (true, _) => ([0.0; 3], [0.0; 3]),
        (false, true) => ([1.0; 3], [1.0, 0.271, 0.227]),
        (false, false) => ([1.0; 3], [1.0; 3]),
    };
    let y0 = (h_pt - BODY_H) / 2.0;
    let outline_a = if level.is_none() { UNKNOWN_ALPHA } else { 1.0 };
    let fill_a = if stale { STALE_ALPHA } else { 1.0 };

    let half = STROKE / 2.0;
    let outline = coverage(w, h, |pm, paint| {
        let path = rounded_rect(half, y0 + half, BODY_W - STROKE, BODY_H - STROKE, BODY_RADIUS - half).expect("body path");
        let stroke = Stroke { width: STROKE, ..Stroke::default() };
        pm.stroke_path(&path, paint, &stroke, Transform::from_scale(s, s), None);
    });
    let nub = coverage(w, h, |pm, paint| {
        let path = rounded_rect(BODY_W + NUB_GAP, (h_pt - NUB_H) / 2.0, NUB_W, NUB_H, NUB_W / 2.0).expect("nub path");
        pm.fill_path(&path, paint, FillRule::Winding, Transform::from_scale(s, s), None);
    });
    let fw = level.map_or(0.0, fill_width_pt);
    let fill = coverage(w, h, |pm, paint| {
        if fw > 0.0 {
            let path = rounded_rect(FILL_INSET, y0 + FILL_INSET, fw, BODY_H - 2.0 * FILL_INSET, FILL_RADIUS).expect("fill path");
            pm.fill_path(&path, paint, FillRule::Winding, Transform::from_scale(s, s), None);
        }
    });
    let bolt_cov = coverage(w, h, |pm, paint| {
        if v.live().is_some() && v.charging {
            let path = bolt(BODY_W / 2.0, h_pt / 2.0, 1.0).expect("bolt path");
            pm.fill_path(&path, paint, FillRule::Winding, Transform::from_scale(s, s), None);
        }
    });

    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for i in 0..(w * h) as usize {
        // bolt is knocked out of the fill (xor of coverages); where there is no fill it stays solid
        let f = fill[i] * fill_a;
        let b = bolt_cov[i];
        let fa = f * (1.0 - b) + b * (1.0 - fill[i]);
        let oa = (outline[i] * outline_a).max(nub[i] * NUB_ALPHA * outline_a);
        // source-over of the fill layer on the outline layer (straight alpha)
        let a = fa + oa * (1.0 - fa);
        let c = |k: usize| if a > 0.0 { (fill_rgb[k] * fa + base[k] * oa * (1.0 - fa)) / a } else { 0.0 };
        rgba.extend([c(0), c(1), c(2), a].map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8));
    }
    Glyph { rgba, w, h }
}
