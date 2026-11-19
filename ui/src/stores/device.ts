// The one place that talks to the backend. Holds the snapshot, keeps it in sync with device events and
// owns the apply dock state (what is on the mouse, what is still only in the profile).

import { defineStore } from "pinia";
import { computed, reactive, ref } from "vue";
import { api, on } from "../api/client";
import type {
  ActionCatalog, ActionRef, ApplyProgress, ApplyResult, DeviceStatus, DpiEncoding, DpiVerification, Macro, MacroInput,
  Profile, ProfileSummary, SettingsPatch, SettingsResult, SkippedGroup, Snapshot,
} from "../api/client";
import { type Caps, errorCode, modelOf } from "../lib/caps";
import { t } from "../i18n";
import { errorText } from "../lib/errors";
import { fmt } from "../lib/styles";
import { readBackPatch } from "../lib/readBack";
import { useUi } from "./ui";

export type DockState = "idle" | "writing" | "done" | "failed" | "blocked";
export type GroupId = "sensor" | "lightpower" | "polling" | "buttons" | "macros";

const GROUP_OF: Record<keyof SettingsPatch, GroupId> = {
  dpiStages: "sensor", activeStage: "sensor", lod: "sensor", rippleControl: "sensor", angleSnap: "sensor", motionSync: "sensor",
  polling: "polling", lighting: "lightpower", sleepMinutes: "lightpower", deepSleepMinutes: "lightpower", debounce: "lightpower",
};


/** Merge `b` into `a` (stages by index, lighting field by field). */
function mergePatch(a: SettingsPatch, b: SettingsPatch): SettingsPatch {
  const stages = new Map((a.dpiStages ?? []).map((s) => [s.index, { ...s }]));
  for (const s of b.dpiStages ?? []) stages.set(s.index, { ...stages.get(s.index), ...s });
  const out: SettingsPatch = { ...a, ...b };
  if (stages.size) out.dpiStages = [...stages.values()];
  if (a.lighting || b.lighting) out.lighting = { ...a.lighting, ...b.lighting };
  return out;
}

function applyLocal(p: Profile, patch: SettingsPatch): void {
  const { dpiStages, lighting, ...flat } = patch;
  for (const s of dpiStages ?? []) {
    const st = p.dpiStages[s.index];
    if (st) Object.assign(st, Object.fromEntries(Object.entries(s).filter(([, v]) => v !== undefined)));
  }
  if (lighting) Object.assign(p.lighting, Object.fromEntries(Object.entries(lighting).filter(([, v]) => v !== undefined)));
  Object.assign(p, Object.fromEntries(Object.entries(flat).filter(([, v]) => v !== undefined)));
}

export const useDevice = defineStore("device", () => {
  const ui = useUi();
  const ready = ref(false);
  const loadError = ref<string | null>(null);
  const status = ref<DeviceStatus | null>(null);
  const profile = ref<Profile | null>(null);
  const profiles = ref<ProfileSummary[]>([]);
  const macros = ref<Macro[]>([]);
  const caps = ref<Caps | null>(null);
  const catalog = ref<ActionCatalog | null>(null);
  const pendingChanges = ref(false);
  /** Groups the mouse has not received because apply leaves them out (DPI with an unverified encoding). */
  const pendingGroups = ref<SkippedGroup[]>([]);
  /** Macro ids edited while a button plays them: the button holds the old copy until it is rebound. */
  const staleMacros = ref<number[]>([]);
  /** Stored verdict of the DPI encoding check; null before any check. */
  const dpiEncoding = ref<DpiEncoding | null>(null);
  const dock = reactive({
    state: "idle" as DockState,
    packets: 0,
    attempt: null as number | null,
    message: "",
    /** "unverified" marks the DPI-encoding refusal so the dock can point to the DPI screen */
    kind: "" as "" | "unverified",
    groups: [] as GroupId[],
  });
  let announce: string | null = null;

  const model = computed(() => (caps.value ? modelOf(caps.value) : null));
  const linked = computed(() => !!status.value?.connected);
  const awake = computed(() => !!status.value?.connected && status.value.awake);
  const activeStage = computed(() => profile.value?.dpiStages[profile.value.activeStage] ?? null);
  const dirtyGroups = computed<GroupId[]>(() => {
    const g = [...dock.groups];
    if (staleMacros.value.length && !g.includes("macros")) g.push("macros");
    return g;
  });
  const hasPending = computed(() => pendingChanges.value || staleMacros.value.length > 0);
  const dpiHeld = computed(() => pendingGroups.value.some((g) => g.group === "dpi"));

  function take(s: Snapshot) {
    status.value = s.status;
    profile.value = s.profile;
    profiles.value = s.profiles;
    macros.value = s.macros;
    caps.value = s.capabilities;
    pendingChanges.value = s.pendingChanges;
    pendingGroups.value = s.pendingGroups ?? [];
    dpiEncoding.value = s.dpiEncoding ?? null;
  }

  function fail(e: unknown) {
    const text = errorText(e, model.value?.name);
    if (errorCode(e) === "unverified_encoding") {
      Object.assign(dock, { state: "failed", kind: "unverified", message: text });
    } else ui.toast("error", text);
  }

  async function refresh() {
    try {
      take(await api.getSnapshot());
    } catch (e) {
      fail(e);
    }
  }

  const skippedDpi = (g: SkippedGroup[] | undefined) => !!g?.some((x) => x.group === "dpi");

  function onApply(r: ApplyResult) {
    if (skippedDpi(r.skippedGroups)) pendingGroups.value = r.skippedGroups;
    if (r.status === "queued") Object.assign(dock, { state: "writing", packets: r.packets || dock.packets, attempt: null, kind: "" });
    else if (r.status === "asleep") Object.assign(dock, { state: "blocked", message: t("dock.asleepApply"), kind: "" });
    else if (r.status === "no-device") Object.assign(dock, { state: "idle", kind: "" });
    else if (r.status === "nothing" && dock.state !== "writing") {
      dock.state = "idle";
      pendingChanges.value = false;
    }
    if (r.status !== "nothing") pendingChanges.value = true;
  }

  function onProgress(p: ApplyProgress) {
    if (p.state === "pending") Object.assign(dock, { state: "writing", packets: p.packets, attempt: null, kind: "" });
    else if (p.state === "retry") Object.assign(dock, { state: "writing", attempt: p.attempt });
    else if (p.state === "failed") {
      // reason is a log line; people get the count and what to do
      const message = p.failed && p.packets ? t("dock.notConfirmed", { failed: p.failed, n: p.packets }, p.packets) : t("dock.changeNotConfirmed");
      Object.assign(dock, { state: "failed", kind: "", message });
      announce = null;
    } else if (skippedDpi(p.skippedGroups)) {
      // partial success: everything but the DPI packet reached the mouse; pendingChanges stays true
      pendingGroups.value = p.skippedGroups;
      dock.groups = dock.groups.filter((g) => g === "sensor");
      Object.assign(dock, { state: "failed", kind: "unverified", message: t("dock.appliedExceptDpi") });
      announce = null;
    } else {
      pendingChanges.value = false;
      pendingGroups.value = [];
      const skipped = p.skipped > 0 ? ` ${t("dock.bleKeepsPolling")}` : "";
      Object.assign(dock, { state: "done", kind: "", groups: [], message: t("dock.applied") + skipped });
      if (announce) ui.toast("ok", t("toast.profileOnMouse", { name: announce }));
      announce = null;
      setTimeout(() => dock.state === "done" && (dock.state = "idle"), 2600);
    }
  }

  async function init() {
    try {
      const [s, c] = await Promise.all([api.getSnapshot(), api.listActions()]);
      take(s);
      catalog.value = c;
      ready.value = true;
    } catch (e) {
      loadError.value = errorText(e);
      return;
    }
    await on("device-status", (st) => {
      const wasAwake = awake.value;
      const prev = status.value;
      status.value = st;
      // caps (model, polling rates per link) only arrive with a snapshot; re-read when the mouse or link changes
      if (prev && (prev.model !== st.model || prev.mode !== st.mode || prev.connected !== st.connected)) {
        if (prev.model !== st.model) dpiResult.value = null;
        void flush().then(refresh);
      }
      if (st.connected && st.awake && !wasAwake && dock.state === "blocked") dock.state = "idle";
      if (!st.connected && dock.state === "writing") Object.assign(dock, { state: "failed", kind: "", message: t("dock.disconnected") });
    });
    await on("dpi-stage-changed", ({ stage, dpi }) => {
      const p = profile.value;
      if (!p) return;
      p.activeStage = stage;
      ui.stagePulse++;
      ui.toast("ok", t("toast.stageChanged", { stage: stage + 1, dpi: fmt(dpi) }), p.dpiStages[stage]?.color);
    });
    await on("apply-progress", onProgress);
    await on("dpi-verification", (ev) => {
      if (ev.state === "running") verifyingDpi.value = true;
      else void settleDpiCheck(ev.result);
    });
  }

  // ---- settings: local first, then the backend (deferred for slider drags) ----------------
  let queued: SettingsPatch | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;

  function markGroups(patch: SettingsPatch) {
    for (const k of Object.keys(patch) as (keyof SettingsPatch)[]) {
      const g = GROUP_OF[k];
      if (g && !dock.groups.includes(g)) dock.groups.push(g);
    }
  }

  async function send(patch: SettingsPatch) {
    try {
      const r: SettingsResult = await api.updateSettings(patch);
      profile.value = r.profile;
      onApply(r.apply);
    } catch (e) {
      fail(e);
      await refresh();
    }
  }

  function flush() {
    if (timer) clearTimeout(timer);
    timer = null;
    const p = queued;
    queued = null;
    return p ? send(p) : Promise.resolve();
  }

  /** Change settings. `defer` coalesces rapid changes (slider drags, typing) into one write. */
  function update(patch: SettingsPatch, defer = false) {
    if (!profile.value) return;
    applyLocal(profile.value, patch);
    markGroups(patch);
    queued = queued ? mergePatch(queued, patch) : patch;
    if (timer) clearTimeout(timer);
    if (defer) timer = setTimeout(flush, 280);
    else void flush();
  }

  async function setButton(button: number, action: ActionRef) {
    try {
      await flush();
      const r = await api.setButtonAction(button, action);
      profile.value = r.profile;
      if (!dock.groups.includes("buttons")) dock.groups.push("buttons");
      if (action.kind === "macro") staleMacros.value = staleMacros.value.filter((id) => id !== action.macroId);
      onApply(r.apply);
      return true;
    } catch (e) {
      fail(e);
      return false;
    }
  }

  async function applyAll() {
    await flush();
    if (!linked.value) return ui.toast("error", t("toast.noMouse"));
    // edited macros reach the mouse only by binding them again
    const rebind = (profile.value?.buttons ?? []).filter((b) => b.action.kind === "macro" && staleMacros.value.includes(b.action.macroId ?? -1));
    for (const b of rebind) await setButton(b.button, b.action);
    try {
      onApply(await api.applyAll());
    } catch (e) {
      fail(e);
    }
  }

  // ---- profiles ------------------------------------------------------------------------
  async function profileCall(run: () => Promise<Snapshot>, thenApply = false) {
    try {
      await flush();
      take(await run());
      dock.groups = [];
      staleMacros.value = [];
      if (thenApply && awake.value) {
        announce = profile.value?.name ?? null;
        onApply(await api.applyAll());
      }
      return true;
    } catch (e) {
      fail(e);
      return false;
    }
  }
  const selectProfile = (id: number) => profileCall(() => api.selectProfile(id), true);
  const createProfile = (name: string) => profileCall(() => api.createProfile(name), true);
  const renameProfile = (id: number, name: string) => profileCall(() => api.renameProfile(id, name));
  const deleteProfile = (id: number) => profileCall(() => api.deleteProfile(id), true);
  const importProfile = (path: string) => profileCall(() => api.importProfile(path), true);

  async function exportProfile(id: number, path: string) {
    try {
      await api.exportProfile(id, path);
      ui.toast("ok", t("toast.exported", { name: profiles.value.find((p) => p.id === id)?.name ?? "" }));
    } catch (e) {
      fail(e);
    }
  }

  async function resetProfile(id: number) {
    try {
      await flush();
      const r = await api.resetProfile(id, true);
      if (id === profile.value?.id) profile.value = r.profile;
      ui.toast("ok", t("toast.reset", { name: r.profile.name }));
      onApply(r.apply);
    } catch (e) {
      fail(e);
    }
  }

  // ---- macros --------------------------------------------------------------------------
  async function macroCall(run: () => Promise<Macro[]>): Promise<boolean> {
    try {
      macros.value = await run();
      return true;
    } catch (e) {
      fail(e);
      return false;
    }
  }
  const createMacro = (input: MacroInput) => macroCall(() => api.createMacro(input));
  async function updateMacro(id: number, input: MacroInput) {
    const ok = await macroCall(() => api.updateMacro(id, input));
    const bound = profile.value?.buttons.some((b) => b.action.kind === "macro" && b.action.macroId === id);
    if (ok && bound && !staleMacros.value.includes(id)) staleMacros.value.push(id);
    return ok;
  }
  async function deleteMacro(id: number) {
    const ok = await macroCall(() => api.deleteMacro(id));
    if (ok) {
      staleMacros.value = [];
      await refresh();
    }
    return ok;
  }

  // ---- read back: what the mouse holds lands in the profile as unapplied changes -----------
  const reading = ref(false);
  const onCableOrReceiver = computed(() => !!status.value?.connected && (status.value.mode === "wired" || status.value.mode === "dongle"));
  const canReadBack = computed(() => !!caps.value?.supportsReadBack && onCableOrReceiver.value);
  async function readBack() {
    if (!profile.value || reading.value) return;
    reading.value = true;
    try {
      await flush();
      const { patch, count } = readBackPatch(await api.readBack(), profile.value);
      if (!count) ui.toast("ok", t("toast.readMatches"));
      else {
        update(patch);
        ui.toast("ok", t("toast.readLoaded", { n: count }, count));
      }
    } catch (e) {
      ui.toast("error", errorText(e, model.value?.name));
    } finally {
      reading.value = false;
    }
  }

  // ---- DPI encoding check (manual or automatic on cable/receiver); the snapshot after it says whether DPI is enabled ----
  const verifyingDpi = ref(false);
  const dpiResult = ref<DpiVerification | null>(null);
  const canVerifyDpi = computed(() => onCableOrReceiver.value);
  /** Why DPI is still locked after a check: this session's result first, else the stored verdict. */
  const dpiCheck = computed<{ status: "inconclusive" | "timeout" | "mismatch"; reason?: string } | null>(() => {
    if (caps.value?.dpiVerified) return null;
    const r = dpiResult.value;
    if (r?.status === "unsupported_link") return null; // the banner already asks for the cable or receiver
    if (r) return { status: r.status === "verified" ? "mismatch" : r.status, reason: r.reason };
    const v = dpiEncoding.value;
    return v ? { status: v.verified ? "mismatch" : "inconclusive" } : null;
  });
  // Runs for the command's answer and for the "done" event (whichever lands first); the toast fires once on the change.
  async function settleDpiCheck(r: DpiVerification | undefined) {
    const was = !!caps.value?.dpiVerified;
    await flush();
    await refresh();
    verifyingDpi.value = false;
    if (r) dpiResult.value = r;
    if (!was && caps.value?.dpiVerified) ui.toast("ok", t("toast.dpiVerified", { model: model.value?.name ?? "" }));
  }
  async function verifyDpi() {
    if (verifyingDpi.value) return;
    verifyingDpi.value = true;
    dpiResult.value = null;
    try {
      await settleDpiCheck(await api.verifyDpiEncoding());
    } catch (e) {
      verifyingDpi.value = false;
      ui.toast("error", errorText(e, model.value?.name));
    }
  }

  return {
    ready, loadError, status, profile, profiles, macros, caps, catalog, pendingChanges, pendingGroups, staleMacros, dock,
    model, linked, awake, activeStage, dirtyGroups, hasPending, dpiHeld,
    init, refresh, update, flush, setButton, applyAll, reading, canReadBack, readBack,
    onCableOrReceiver, canVerifyDpi, verifyingDpi, dpiCheck, verifyDpi,
    selectProfile, createProfile, renameProfile, deleteProfile, importProfile, exportProfile, resetProfile,
    createMacro, updateMacro, deleteMacro,
  };
});
