// Per-model hero art. X3-shaped models use the drawn illustration (MouseIllustration.vue); the others use
// the vendor's top-view render with its printed number badges removed. Coordinates are source pixels:
// `led` is where the stage light comes from, `spots` are the button centers by ButtonInfo.position.

// The renders are not in git (vendor copyright): a model whose PNG is missing falls back to ILLUSTRATION.
const RENDERS = import.meta.glob<string>("../assets/models/*.png", { eager: true, import: "default" });

export type Point = [number, number];

export interface ModelArt {
  /** null = the drawn X3 illustration */
  src: string | null;
  w: number;
  h: number;
  led: Point;
  spots: Record<string, Point>;
}

export const ILLUSTRATION: ModelArt = {
  src: null,
  w: 400,
  h: 720,
  led: [200, 257],
  spots: { left: [120, 187], right: [280, 187], middle: [200, 144], "side-forward": [44, 259], "side-back": [48, 367] },
};

const art = (file: string, w: number, h: number, led: Point, l: Point, m: Point, r: Point, sf: Point, sb: Point, dpi?: Point): ModelArt => {
  const src = RENDERS[`../assets/models/${file}.png`];
  if (!src) return ILLUSTRATION;
  return { src, w, h, led, spots: { left: l, middle: m, right: r, "side-forward": sf, "side-back": sb, ...(dpi ? { dpi } : {}) } };
};

const R1 = art("r1", 225, 398, [109.5, 108], [54.5, 73.5], [109.5, 68.5], [162.5, 70.5], [15.1, 146.5], [17.8, 211.5], [109.5, 145.5]);
const X11 = art("x11", 215, 398, [106.2, 193.1], [59.5, 83.5], [105.5, 77.5], [157.5, 83.5], [12.2, 155.5], [12.2, 214.5]);
const X1 = art("x1", 250, 400, [124.2, 26.8], [75.5, 73.5], [123.5, 73.5], [173.5, 73.5], [22.4, 152.5], [24.4, 214.5], [124.5, 139.5]);
const V5SE = art("v5-se", 218, 398, [109.7, 161.9], [59.5, 78.5], [109.5, 78.5], [157.5, 78.5], [13.9, 167.5], [13.4, 223.5]);
const X8 = art("x8", 240, 400, [121.2, 22], [71.5, 71.5], [121.5, 71.5], [169.5, 71.5], [24.4, 147.5], [23.4, 203.5]);
const G3 = art("g3", 240, 398, [121.1, 143.7], [71.5, 76.5], [121.5, 76.5], [169.5, 76.5], [12.8, 143.5], [12.3, 199.5]);
const V3 = art("v3", 240, 400, [119.8, 26.5], [68.5, 69.5], [118.5, 69.5], [166.5, 69.5], [24.4, 147.5], [23.4, 203.5]);
const V6 = art("v6", 240, 400, [135.5, 203.8], [85.5, 93.5], [135.5, 93.5], [183.5, 93.5], [32.1, 167.1], [37.5, 221.5]);

/** Keyed by the slug of ModelSpec.name. Vendor pages: MS3 R1, MS4/7/10 X11 family, MS6 X1, MS9 V5 SE,
 *  MS11-13 X8 family, MS14-15 G3 family, MS16/18 V3 family, MS17 V6. X3, X6, X3 MAX, X3 PRO share the X3 shell. */
const BY_MODEL: Record<string, ModelArt> = {
  r1: R1,
  x11: X11, "x11-pro": X11, "x11-se": X11,
  x1: X1,
  "v5-se": V5SE,
  "x8-se": X8, "x8-plus": X8, "x8-pro": X8,
  g3: G3, "g3-pro": G3,
  v3: V3, "v3-pro": V3,
  v6: V6,
};

export const artFor = (slug: string): ModelArt => BY_MODEL[slug] ?? ILLUSTRATION;

/** Percent position inside the art box, for absolutely placed hotspots. */
export const pct = (a: ModelArt, p: Point): { left: string; top: string } => ({ left: `${(p[0] / a.w) * 100}%`, top: `${(p[1] / a.h) * 100}%` });
