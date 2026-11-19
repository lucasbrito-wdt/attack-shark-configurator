// read_back -> settings patch. Only decoded (non-null) values that differ from the profile go in;
// a field the model does not have (null in the profile) is skipped, and buttons are never read back.
import type { DpiStagePatch, LightingPatch, Profile, ReadBack, SettingsPatch } from "../api/client";

const SCALARS = ["activeStage", "polling", "lod", "rippleControl", "angleSnap", "motionSync", "debounce", "sleepMinutes", "deepSleepMinutes"] as const;

export function readBackPatch(r: ReadBack, p: Profile): { patch: SettingsPatch; count: number } {
  const patch: Record<string, unknown> = {};
  let count = 0;
  for (const k of SCALARS) {
    const v = r[k];
    if (v != null && p[k] != null && v !== p[k]) {
      patch[k] = v;
      count++;
    }
  }
  const stages: DpiStagePatch[] = [];
  for (const s of r.dpiStages) {
    const cur = p.dpiStages.find((x) => x.index === s.index);
    if (!cur) continue;
    const d: DpiStagePatch = { index: s.index };
    if (s.dpi != null && s.dpi !== cur.dpi) d.dpi = s.dpi;
    if (s.enabled != null && s.enabled !== cur.enabled) d.enabled = s.enabled;
    if (s.color != null && s.color.toLowerCase() !== cur.color.toLowerCase()) d.color = s.color;
    const n = Object.keys(d).length - 1;
    if (n) {
      stages.push(d);
      count += n;
    }
  }
  if (stages.length) patch.dpiStages = stages;
  const light: LightingPatch = {};
  for (const k of ["mode", "brightness", "speed"] as const) {
    const v = r.lighting[k];
    if (v != null && v !== p.lighting[k]) light[k] = v;
  }
  if (r.lighting.color != null && r.lighting.color.toLowerCase() !== p.lighting.color.toLowerCase()) light.color = r.lighting.color;
  if (Object.keys(light).length) {
    patch.lighting = light;
    count += Object.keys(light).length;
  }
  return { patch: patch as SettingsPatch, count };
}
