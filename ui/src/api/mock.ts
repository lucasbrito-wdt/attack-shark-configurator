// In-memory backend used when the app runs in a plain browser. Same contract as the Rust commands.
// Control it from the URL (?mock=offline|asleep|wired|ble|ble-nobas|fail|retry|unverified|verify-ok|verify-inconclusive, ?model=<id in mockModels.ts>) or at runtime
// via window.__sharkMock.

import { actionLabel, capabilities, catalog, defaultProfile, keyLabel, models } from "./mockData";
import type {
  ActionRef, ApiError, ApplyProgress, ApplyResult, Battery, Capabilities, CommandMap, CommandName, DeviceStatus, DpiEncoding, DpiVerification,
  EventMap, EventName, LinkMode, Macro,
  MacroInput, Profile, ReadBack, SettingsPatch, SettingsResult, SkippedGroup, Snapshot,
} from "./types";

export type ApplyBehavior = "ok" | "fail" | "retry-then-ok";

const LATENCY_MS = 60;
const ACK_DELAY_MS = 900;

const fail = (code: ApiError["code"], message: string): never => {
  throw { code, message } satisfies ApiError;
};
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const hex = /^#[0-9a-fA-F]{6}$/;

export class MockBackend {
  private profiles: Profile[] = [];
  private selected = 0;
  private macros: Macro[] = [
    { id: 0, name: "Copy and paste", loopMode: "count", loopCount: 1, events: [
      { key: 0xe0, down: true, delayMs: 20 }, { key: 0x06, down: true, delayMs: 30 }, { key: 0x06, down: false, delayMs: 20 },
      { key: 0xe0, down: false, delayMs: 20 },
    ] },
  ];
  private mode: LinkMode = "dongle";
  private awake = true;
  private level: number | null = null;
  /** The BLE Battery Service (0x2A19) answers, like the real X6; ?mock=ble-nobas turns it off. */
  private bas = true;
  private pending = true;
  /** Groups the last apply skipped; the backend keeps them pending until a write succeeds. */
  private pendingGroups: SkippedGroup[] = [];
  private baseCaps: Capabilities = capabilities;
  /** Mirrors SHARK_ALLOW_UNVERIFIED=1; ?mock=unverified turns it off, so applies on the X6 report a skipped DPI group. */
  private allowUnverified = true;
  private lastKnown: Battery["lastKnown"] = { level: 70, charging: false, link: "dongle", at: "2026-11-19T12:00:00Z" };
  private sending = false;
  /** Outcome of the DPI check of an unverified model: ?mock=verify-inconclusive, else verified (the X6 factory block settles u-1). */
  private verifyOutcome: "verified" | "inconclusive" = "verified";
  private dpiEncoding: DpiEncoding | null = null;
  /** How the next apply ends. */
  behavior: ApplyBehavior = "ok";
  private handlers = new Map<EventName, Set<(p: never) => void>>();
  private timers: ReturnType<typeof setInterval>[] = [];

  constructor() {
    const q = typeof location === "undefined" ? "" : new URLSearchParams(location.search).get("mock") ?? "";
    if (q === "offline") this.mode = "none";
    if (q === "wired" || q === "ble") this.mode = q;
    if (q === "ble-nobas") [this.mode, this.bas] = ["ble", false];
    if (q === "asleep") this.awake = false;
    if (q === "fail") this.behavior = "fail";
    if (q === "retry") this.behavior = "retry-then-ok";
    if (q === "unverified") this.allowUnverified = false;
    if (q === "verify-ok" || q === "verify-inconclusive") [this.allowUnverified, this.verifyOutcome] = [false, q === "verify-ok" ? "verified" : "inconclusive"];
    const model = typeof location === "undefined" ? null : new URLSearchParams(location.search).get("model");
    if (model && models[model]) this.baseCaps = models[model];
    this.profiles = [this.fresh(0, "Default"), this.fresh(1, "FPS")];
    if (typeof window !== "undefined") (window as unknown as { __sharkMock: MockBackend }).__sharkMock = this;
  }

  /** Capabilities on the current link: XVI wired is capped at 1000 Hz, like the backend; a stored verified DPI check sets dpiVerified. */
  private get caps(): Capabilities {
    const c = this.baseCaps;
    const v = this.dpiEncoding;
    const dpiVerified = c.dpiVerified || (v !== null && v.verified && v.format === c.dpiEncoding);
    const pollingRates = c.platform === "xvi" && this.mode === "wired" ? c.pollingRates.filter((r) => r <= 1000) : c.pollingRates;
    return { ...c, pollingRates, dpiVerified };
  }

  /** Default profile with null for the settings this model does not have (as the backend reports it). */
  private fresh(id: number, name: string): Profile {
    const p = defaultProfile(id, name);
    const c = this.baseCaps;
    return {
      ...p,
      rippleControl: c.supportsRippleControl ? p.rippleControl : null,
      angleSnap: c.supportsAngleSnap ? p.angleSnap : null,
      motionSync: c.supportsMotionSync ? p.motionSync : null,
      debounce: c.debounce ? p.debounce : null,
      sleepMinutes: c.sleepMinutes ? Math.min(p.sleepMinutes ?? 1, c.sleepMinutes.max) : null,
      deepSleepMinutes: c.deepSleepMinutes ? p.deepSleepMinutes : null,
    };
  }

  // ---- control hooks for design work ---------------------------------------------------
  setMode(mode: LinkMode, awake = true): void {
    this.mode = mode;
    this.awake = awake;
    if (mode === "none") this.level = null;
    this.emitStatus();
  }

  // ---- events --------------------------------------------------------------------------
  listen<E extends EventName>(event: E, handler: (payload: EventMap[E]) => void): () => void {
    const set = this.handlers.get(event) ?? new Set();
    set.add(handler as (p: never) => void);
    this.handlers.set(event, set);
    this.startTimers();
    return () => set.delete(handler as (p: never) => void);
  }

  private emit<E extends EventName>(event: E, payload: EventMap[E]): void {
    this.handlers.get(event)?.forEach((h) => h(payload as never));
  }

  private status(): DeviceStatus {
    const connected = this.mode !== "none";
    // the X6 sends a bogus vendor battery over BLE: the backend falls back to BAS, else unknown
    const bleBlind = this.mode === "ble" && !this.caps.batteryOverBle;
    const vendor = bleBlind ? null : this.level;
    const bas = this.mode === "ble" && this.bas ? this.level : null;
    const level = vendor ?? bas;
    return {
      connected,
      mode: this.mode,
      model: connected ? (this.caps.modelId === "x6" ? "X6 Mouse2" : this.caps.modelName) : null,
      awake: connected && this.awake,
      battery: {
        level,
        charging: connected && !bleBlind ? this.mode === "wired" : null,
        source: vendor !== null ? "vendor-event" : bas !== null ? "ble-bas" : "none",
        reason: bleBlind && bas === null ? "not-reported-over-ble" : null,
        lastKnown: this.lastKnown,
      },
    };
  }

  private emitStatus(): void {
    this.emit("device-status", this.status());
  }

  private startTimers(): void {
    if (this.timers.length) return;
    // the backend checks an unverified model's DPI encoding once the cable or receiver is up
    this.timers.push(setTimeout(() => {
      const usb = this.mode === "wired" || this.mode === "dongle";
      if (usb && !this.caps.dpiVerified && !this.dpiEncoding) this.cmd_verify_dpi_encoding().catch(() => undefined);
    }, 1000) as never);
    // first heartbeat after 2 s (battery stays null until then), then every 5 s; 10 % drop every ~minute
    const heartbeat = () => {
      if (this.mode === "none" || !this.awake) return;
      this.level ??= 80;
      if (this.mode !== "ble" || this.caps.batteryOverBle || this.bas) {
        const link = this.mode === "wired" ? "usb" : this.mode;
        this.lastKnown = { level: this.level, charging: this.mode === "wired", link, at: new Date().toISOString().replace(/\.\d+Z$/, "Z") };
      }
      this.emitStatus();
    };
    this.timers.push(setTimeout(heartbeat, 2000) as never, setInterval(heartbeat, 5000));
    this.timers.push(setInterval(() => {
      if (this.level !== null && this.level > 10 && Math.random() < 0.1) this.level -= 10;
    }, 5000));
    // the mouse button cycles the DPI stage every 12 s
    this.timers.push(setInterval(() => {
      const p = this.profiles[this.selected];
      if (this.mode === "none" || !this.awake) return;
      const on = p.dpiStages.filter((s) => s.enabled && s.dpi > 0);
      if (on.length < 2) return;
      const next = on[(on.findIndex((s) => s.index === p.activeStage) + 1) % on.length];
      p.activeStage = next.index;
      this.emit("dpi-stage-changed", { stage: next.index, dpi: next.dpi });
    }, 12000));
  }

  // ---- apply ---------------------------------------------------------------------------
  private apply(full: boolean, resync = false): ApplyResult {
    if (this.mode === "none") return { status: "no-device", packets: 0, skippedGroups: [] };
    if (!this.awake) return { status: "asleep", packets: 0, skippedGroups: [] };
    if (this.sending) return { status: "queued", packets: 0, skippedGroups: [] };
    // the backend leaves out only the DPI packet of an unverified model and sends the rest
    const skippedGroups: SkippedGroup[] = !this.caps.dpiVerified && !this.allowUnverified ? [{ group: "dpi", code: "unverified_encoding" }] : [];
    if (!this.pending && !full && !resync) return { status: "nothing", packets: 0, skippedGroups: [] };
    const packets = full || resync ? 5 : 2;
    this.sending = true;
    const progress = (p: Partial<ApplyProgress> & Pick<ApplyProgress, "state">) =>
      this.emit("apply-progress", { packets, skipped: 0, failed: 0, attempt: null, reason: null, skippedGroups, unconfirmed: false, ...p });
    progress({ state: "pending" });
    void (async () => {
      if (this.behavior === "retry-then-ok") {
        for (const attempt of [1, 2]) {
          await sleep(ACK_DELAY_MS / 2);
          progress({ state: "retry", attempt });
        }
      }
      await sleep(ACK_DELAY_MS);
      this.sending = false;
      if (this.behavior === "fail") {
        progress({ state: "failed", failed: 1, reason: "1 packet(s) not delivered" });
      } else {
        const ble = this.mode === "ble";
        this.pendingGroups = skippedGroups;
        this.pending = ble || skippedGroups.length > 0;
        progress({ state: "ok", skipped: ble ? 1 : 0, reason: ble ? "polling rate is not sent over Bluetooth" : null, unconfirmed: ble });
      }
    })();
    return { status: "queued", packets, skippedGroups };
  }

  // ---- commands ------------------------------------------------------------------------
  async invoke<K extends CommandName>(cmd: K, args: CommandMap[K]["args"]): Promise<CommandMap[K]["result"]> {
    await sleep(LATENCY_MS);
    const a = args as never as Record<string, never>;
    const handler = (this as unknown as Record<string, (a: Record<string, never>) => unknown>)[`cmd_${cmd}`];
    return structuredClone(await handler.call(this, a)) as CommandMap[K]["result"];
  }

  private cur(): Profile {
    return this.profiles[this.selected];
  }

  private reindex(): void {
    this.profiles.forEach((p, i) => (p.id = i));
  }

  private snapshot(): Snapshot {
    return {
      status: this.status(),
      profile: this.cur(),
      profiles: this.profiles.map((p) => ({ id: p.id, name: p.name })),
      macros: this.macros,
      capabilities: this.caps,
      pendingChanges: this.pending,
      pendingGroups: this.pendingGroups,
      dpiEncoding: this.dpiEncoding,
    };
  }

  private name(n: string): string {
    const t = n.trim();
    if (!t || [...t].length > 48) fail("invalid_input", "name must have 1..=48 characters");
    return t;
  }

  private profile(id: number): Profile {
    return this.profiles[id] ?? fail("not_found", "profile not found");
  }

  private cmd_get_snapshot = () => this.snapshot();
  private cmd_list_profiles = () => this.snapshot().profiles;
  private cmd_create_profile = ({ name }: { name: string }) => {
    this.profiles.push(this.fresh(this.profiles.length, this.name(name)));
    this.selected = this.profiles.length - 1;
    this.pending = true;
    return this.snapshot();
  };
  private cmd_rename_profile = ({ id, name }: { id: number; name: string }) => {
    this.profile(id).name = this.name(name);
    return this.snapshot();
  };
  private cmd_delete_profile = ({ id }: { id: number }) => {
    this.profile(id);
    if (this.profiles.length === 1) fail("last_profile", "keep at least one profile");
    this.profiles.splice(id, 1);
    if (id < this.selected) this.selected -= 1;
    else if (id === this.selected) this.pending = true;
    this.selected = Math.min(this.selected, this.profiles.length - 1);
    this.reindex();
    return this.snapshot();
  };
  private cmd_select_profile = ({ id }: { id: number }) => {
    this.profile(id);
    this.selected = id;
    this.pending = true;
    return this.snapshot();
  };
  private cmd_import_profile = ({ path }: { path: string }) => {
    if (!path) fail("invalid_input", "empty path");
    this.profiles.push(this.fresh(this.profiles.length, "Imported"));
    this.selected = this.profiles.length - 1;
    this.pending = true;
    return this.snapshot();
  };
  private cmd_export_profile = ({ id, path }: { id: number; path: string }) => {
    this.profile(id);
    if (!path) fail("invalid_input", "empty path");
    return null;
  };
  private cmd_reset_profile = ({ id, confirm }: { id: number; confirm: boolean }): SettingsResult => {
    if (!confirm) fail("confirm_required", "reset sends the 0x0C resync command and needs confirm: true");
    if (this.sending) fail("busy", "an apply is in progress");
    const fresh = this.fresh(id, this.profile(id).name);
    this.profiles[id] = fresh;
    this.pending = true;
    return { profile: fresh, apply: id === this.selected ? this.apply(true, true) : { status: "nothing", packets: 0, skippedGroups: [] } };
  };
  private cmd_update_settings = ({ patch }: { patch: SettingsPatch }): SettingsResult => {
    this.patch(this.cur(), patch);
    this.pending = true;
    return { profile: this.cur(), apply: this.apply(false) };
  };
  private cmd_set_button_action = ({ button, action: act }: { button: number; action: ActionRef }): SettingsResult => {
    const action = act;
    let bound: ActionRef = action;
    if (!Number.isInteger(button) || button < 0 || button >= 18) fail("invalid_input", "button must be 0..=17");
    let label: string;
    if (action.kind === "preset") {
      if (!catalog.categories.some((c) => c.actions.some((x) => x.id === action.id))) fail("invalid_input", "unknown preset action");
      label = actionLabel(action.id);
    } else if (action.kind === "shortcut") {
      if (!action.key || action.mods > 15) fail("invalid_input", "shortcut needs a key and mods in 0..=15");
      const mods = catalog.modifiers.filter((m) => action.mods & m.bit).map((m) => m.label.split("/")[0]);
      label = `Shortcut: ${[...mods, keyLabel(action.key)].join("+")}`;
    } else {
      const m = this.macros.find((x) => x.id === action.macroId) ?? fail("not_found", "macro not found");
      bound = { kind: "macro", macroId: m.id, name: m.name };
      label = `Macro: ${m.name}`;
    }
    this.cur().buttons[button] = { button, action: bound, label };
    this.pending = true;
    return { profile: this.cur(), apply: this.apply(false) };
  };
  private cmd_list_actions = () => catalog;
  private cmd_list_macros = () => this.macros;
  private cmd_create_macro = ({ input }: { input: MacroInput }) => {
    this.macros.push({ id: this.macros.length, ...this.macro(input) });
    return this.macros;
  };
  private cmd_update_macro = ({ id, input }: { id: number; input: MacroInput }) => {
    const m = this.macros[id] ?? fail("not_found", "macro not found");
    Object.assign(m, this.macro(input));
    return this.macros;
  };
  private cmd_delete_macro = ({ id }: { id: number }) => {
    if (!this.macros[id]) fail("not_found", "macro not found");
    this.macros.splice(id, 1);
    this.macros.forEach((m, i) => (m.id = i));
    return this.macros;
  };
  private cmd_apply_all = () => this.apply(true);
  /** The mock device holds the selected profile; fields the model lacks read back as null. */
  private cmd_read_back = (): ReadBack => {
    if (!this.caps.supportsReadBack) fail("unsupported", `${this.caps.modelName} has no read-back`);
    if (this.mode === "none" || this.mode === "ble") fail("unsupported", "read-back needs the cable or the 2.4G receiver");
    if (this.sending) fail("busy", "an apply is in progress");
    const { id: _i, name: _n, lighting: l, ...p } = structuredClone(this.cur());
    const mode = this.caps.lightingModes[l.mode];
    return {
      ...p,
      firmware: ["f1", "x11-pro-v2"].includes(this.caps.modelId) ? null : 0x1005,
      lod: this.caps.lodOptions.length ? p.lod : null,
      lighting: { ...l, brightness: !mode || mode.usesBrightness ? l.brightness : null },
      buttons: null,
    };
  };

  /** Same outcomes as the backend check; the read takes 1.5 s and holds applies like the backend. */
  private cmd_verify_dpi_encoding = async (): Promise<DpiVerification> => {
    const done = (result: DpiVerification) => (this.emit("dpi-verification", { state: "done", result }), result);
    const c = this.baseCaps;
    if (c.dpiVerified) return done({ status: "verified", format: c.dpiEncoding, reason: "the model's DPI encoding is verified on hardware" });
    if (this.mode === "none") return done({ status: "unsupported_link", reason: "connect the cable or the 2.4G receiver" });
    if (this.mode === "ble") {
      return done({ status: "unsupported_link", reason: "the Bluetooth read-back blob (family-b 4.3) is not decoded; connect the cable or the 2.4G receiver" });
    }
    if (this.sending) fail("busy", "an apply or a read is in progress");
    this.sending = true;
    this.emit("dpi-verification", { state: "running" });
    await sleep(1500);
    this.sending = false;
    const ok = this.verifyOutcome === "verified" && c.platform === "bk";
    this.dpiEncoding = { verified: ok, format: ok ? c.dpiEncoding : null, at: new Date().toISOString().replace(/\.\d+Z$/, "Z"), firmware: 0x1003 };
    if (ok) return done({ status: "verified", format: c.dpiEncoding, reason: "stage 6 holds 07 02, which no R1 table row produces; it is a valid u-1 value" });
    const reason = c.platform !== "bk" ? `${c.modelName} has no read-only DPI check`
      : "every stored DPI stage fits both encodings (u-1 reads 800, 1600, 2400 DPI, the R1 table reads 650, 1350, 2000 DPI)";
    return done({ status: "inconclusive", reason });
  };

  private macro(i: MacroInput): Omit<Macro, "id"> {
    this.name(i.name);
    if (i.events.length > this.caps.maxMacroEvents) fail("invalid_input", `a macro holds at most ${this.caps.maxMacroEvents} events`);
    if (i.events.some((e) => !e.key) || i.loopCount < 1) fail("invalid_input", "events need a key and loopCount must be at least 1");
    return { name: i.name.trim(), events: i.events, loopMode: i.loopMode, loopCount: i.loopCount };
  }

  /** Validates everything first; the profile changes only if the whole patch is valid. */
  private patch(p: Profile, patch: SettingsPatch): void {
    const n = structuredClone(p);
    const c = this.caps;
    const range = (name: string, v: number | undefined, min: number, max: number) => {
      if (v !== undefined && !(Number.isInteger(v) && v >= min && v <= max)) fail("invalid_input", `${name} must be ${min}..=${max}`);
    };
    for (const s of patch.dpiStages ?? []) {
      const st = n.dpiStages[s.index] ?? fail("invalid_input", "dpi stage index must be 0..=7");
      if (s.dpi !== undefined) {
        const step = c.dpiHighStepFrom !== null && s.dpi > c.dpiHighStepFrom ? 100 : c.dpiStep;
        if (s.dpi < c.dpiMin || s.dpi > c.dpiMax || s.dpi % step) fail("invalid_input", `dpi must be ${c.dpiMin}..=${c.dpiMax} in steps of ${c.dpiStep} (100 above ${c.dpiHighStepFrom ?? c.dpiMax})`);
        st.dpi = s.dpi;
      }
      if (s.enabled !== undefined) st.enabled = s.enabled;
      if (s.color !== undefined) st.color = hex.test(s.color) ? s.color.toUpperCase() : fail("invalid_input", "invalid color, expected #RRGGBB");
    }
    range("activeStage", patch.activeStage, 0, 7);
    const has = (name: string, v: unknown, ok: boolean) => {
      if (v !== undefined && !ok) fail("invalid_input", `this model has no ${name} setting`);
    };
    has("rippleControl", patch.rippleControl, c.supportsRippleControl);
    has("angleSnap", patch.angleSnap, c.supportsAngleSnap);
    has("motionSync", patch.motionSync, c.supportsMotionSync);
    for (const [name, r] of [["debounce", c.debounce], ["sleepMinutes", c.sleepMinutes], ["deepSleepMinutes", c.deepSleepMinutes]] as const) {
      const v = patch[name];
      has(name, v, r !== null);
      if (r) range(name, v, r.min, r.max);
    }
    if (patch.polling !== undefined && !c.pollingRates.includes(patch.polling)) fail("invalid_input", "unsupported polling rate");
    if (patch.lod !== undefined && !c.lodOptions.includes(patch.lod)) fail("invalid_input", "unsupported lod");
    const l = patch.lighting;
    range("lighting.mode", l?.mode, 0, c.lightingModes.length - 1);
    range("lighting.brightness", l?.brightness, c.brightness.min, c.brightness.max);
    range("lighting.speed", l?.speed, c.lightSpeed.min, c.lightSpeed.max);
    if (l?.color !== undefined && !hex.test(l.color)) fail("invalid_input", "invalid color, expected #RRGGBB");
    const { dpiStages: _d, lighting: _l, ...flat } = patch;
    Object.assign(n, Object.fromEntries(Object.entries(flat).filter(([, v]) => v !== undefined)));
    Object.assign(n.lighting, Object.fromEntries(Object.entries(l ?? {}).filter(([, v]) => v !== undefined)));
    Object.assign(p, n);
  }
}
