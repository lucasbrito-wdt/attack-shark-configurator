// Helpers over the multi-model Capabilities (crates/shark-core ModelSpec): DPI stepping, slider mapping, button order.

import type { ApiError, ButtonInfo, Capabilities } from "../api/client";

export type Caps = Capabilities;

export interface Model {
  name: string;
  slug: string;
  platform: string;
  dpiVerified: boolean;
  readBack: boolean;
  batteryOverBle: boolean;
}

export function modelOf(caps: Caps): Model {
  return {
    name: caps.modelName,
    slug: caps.modelId,
    platform: caps.platform,
    dpiVerified: caps.dpiVerified,
    readBack: caps.supportsReadBack,
    batteryOverBle: caps.batteryOverBle,
  };
}

/** "t" encoding: the multiplier bits above this value are copied from the vendor, so the backend refuses those writes. */

export const unverifiedFrom = (caps: Caps): number | null => (caps.dpiEncoding === "t" ? 10000 : null);

/** DPI step at a value: dpiStep up to dpiHighStepFrom, 100 above it (family B "T"/"H" encodings). */
export function stepAt(caps: Caps, dpi: number): number {
  return caps.dpiHighStepFrom != null && dpi >= caps.dpiHighStepFrom ? 100 : caps.dpiStep;
}

export function snapDpi(caps: Caps, value: number): number {
  const v = Math.min(caps.dpiMax, Math.max(caps.dpiMin, value));
  const step = stepAt(caps, v);
  return Math.min(caps.dpiMax, Math.max(caps.dpiMin, Math.round(v / step) * step));
}

/** Slider position 0..1000. Piecewise: 50..3200 gets 64 % of the travel, where nearly every player sets stages. */
const KNEE_POS = 640;
const knee = (caps: Caps) => Math.min(3200, Math.round(caps.dpiMax / 2));
export function posToDpi(caps: Caps, p: number): number {
  const k = knee(caps);
  const raw = p <= KNEE_POS ? caps.dpiMin + (p / KNEE_POS) * (k - caps.dpiMin) : k + ((p - KNEE_POS) / (1000 - KNEE_POS)) * (caps.dpiMax - k);
  return snapDpi(caps, raw);
}
export function dpiToPos(caps: Caps, d: number): number {
  const k = knee(caps);
  return Math.round(d <= k ? ((d - caps.dpiMin) / (k - caps.dpiMin)) * KNEE_POS : KNEE_POS + ((d - k) / (caps.dpiMax - k)) * (1000 - KNEE_POS));
}

const BUTTON_ORDER = ["left", "right", "middle", "side-forward", "side-back"];
/** Physical numbering the vendor prints on its renders: 1 L, 2 R, 3 wheel, 4 front side, 5 rear side, then the rest. */
export function orderedButtons(caps: Caps): ButtonInfo[] {
  const rank = (b: ButtonInfo) => {
    const i = BUTTON_ORDER.indexOf(b.position);
    return i < 0 ? BUTTON_ORDER.length + b.id : i;
  };
  return [...caps.buttons].sort((a, b) => rank(a) - rank(b));
}

export function isApiError(e: unknown): e is ApiError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

export const errorCode = (e: unknown): ApiError["code"] => (isApiError(e) ? e.code : "io");
