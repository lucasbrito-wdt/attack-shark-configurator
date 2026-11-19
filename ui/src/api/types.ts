// Mirrors src-tauri/src/dto.rs and crates/shark-core/src/model.rs (camelCase). Keep in sync: Rust tests assert the key names.

export type LinkMode = "wired" | "dongle" | "ble" | "none";
export type BatterySource = "vendor-event" | "ble-bas" | "none";

export interface Battery {
  /** Percent. null until a vendor battery event arrives, or when the link never reports it (see reason). */
  level: number | null;
  /** From the vendor charge state (family A: the wired PID is present). null = unknown. */
  charging: boolean | null;
  source: BatterySource;
  /** Why level is null while connected. */
  reason: "not-reported-over-ble" | null;
  /** Last accepted vendor reading of this model. Persisted: kept across disconnects, restarts and BLE sessions. */
  lastKnown: LastKnownBattery | null;
}

export interface LastKnownBattery {
  level: number;
  charging: boolean;
  /** "usb" = cable. */
  link: "usb" | "dongle" | "ble";
  /** RFC 3339 UTC, e.g. "2026-11-19T14:03:07Z". */
  at: string;
}

export interface DeviceStatus {
  connected: boolean;
  mode: LinkMode;
  /** Name as reported by the device, e.g. "X6 Mouse2". */
  model: string | null;
  awake: boolean;
  battery: Battery;
}

export interface DpiStage {
  index: number;
  /** 0 = unused slot. */
  dpi: number;
  enabled: boolean;
  /** "#RRGGBB" */
  color: string;
}

export interface Lighting {
  mode: number;
  brightness: number;
  speed: number;
  color: string;
}

export type ActionRef =
  | { kind: "preset"; id: number }
  | { kind: "shortcut"; key: number; mods: number }
  /** macroId indexes the macro library; null when the bound copy matches no library entry. name is ignored on input. */
  | { kind: "macro"; macroId: number | null; name?: string | null };

export interface ButtonBinding {
  button: number;
  action: ActionRef;
  label: string;
}

export interface ProfileSummary {
  /** List position. Invalidated by delete_profile: always use the list returned by the last command. */
  id: number;
  name: string;
}

export interface Profile {
  id: number;
  name: string;
  dpiStages: DpiStage[];
  /** 0-based */
  activeStage: number;
  /** Hz */
  polling: number;
  /** Lift-off distance in mm */
  lod: number;
  /** null when the model does not have the setting (capabilities.supports* / range null). */
  rippleControl: boolean | null;
  angleSnap: boolean | null;
  motionSync: boolean | null;
  /** Raw value; ms = debounce * capabilities.debounceMsPerUnit. */
  debounce: number | null;
  sleepMinutes: number | null;
  deepSleepMinutes: number | null;
  lighting: Lighting;
  /** All 18 slots; capabilities.buttons lists the ones the model exposes. */
  buttons: ButtonBinding[];
}

/** read_back result: the Profile shape without id/name; every field null when not decoded. */
export interface ReadBack {
  /** Family B firmware (0x0B); null on C1. */
  firmware: number | null;
  dpiStages: { index: number; dpi: number | null; enabled: boolean | null; color: string | null }[];
  activeStage: number | null;
  polling: number | null;
  lod: number | null;
  rippleControl: boolean | null;
  angleSnap: boolean | null;
  motionSync: boolean | null;
  debounce: number | null;
  sleepMinutes: number | null;
  deepSleepMinutes: number | null;
  lighting: { mode: number | null; brightness: number | null; speed: number | null; color: string | null };
  /** The button map is not decoded: always null. */
  buttons: ButtonBinding[] | null;
}

export type LoopMode = "count" | "untilKey" | "whileHeld";

export interface MacroEvent {
  /** HID keyboard usage, or macro mouse code 0xF1..0xF5 */
  key: number;
  down: boolean;
  delayMs: number;
}

export interface Macro {
  id: number;
  name: string;
  events: MacroEvent[];
  loopMode: LoopMode;
  loopCount: number;
}

export interface MacroInput {
  name: string;
  events: MacroEvent[];
  loopMode: LoopMode;
  loopCount: number;
}

export interface LightingMode {
  id: number;
  label: string;
  /** The device gets the brightness in this mode (otherwise a fixed value): show the slider only when true. */
  usesBrightness: boolean;
  /** Color and speed are always sent; whether the mode uses them is not decoded (null). */
  usesColor: boolean | null;
  usesSpeed: boolean | null;
}

export type ButtonPosition = "left" | "right" | "middle" | "side-back" | "side-forward";

export interface ButtonInfo {
  id: number;
  label: string;
  position: ButtonPosition;
}

export interface Range {
  min: number;
  max: number;
}

export interface Capabilities {
  /** Registry id, e.g. "x6", "x3-max", "f1". */
  modelId: string;
  modelName: string;
  platform: "bk" | "xvi" | "v4";
  /** Vendor model code (BK: low byte of the event type). */
  modelCode: number;
  maxDpiStages: number;
  dpiMin: number;
  dpiMax: number;
  /** Step up to dpiHighStepFrom; 100 above it. */
  dpiStep: number;
  dpiHighStepFrom: number | null;
  dpiEncoding: "a" | "t" | "h" | "direct";
  /** false: apply refuses DPI writes with unverified_encoding unless SHARK_ALLOW_UNVERIFIED=1. */
  dpiVerified: boolean;
  /** Hz, 125..8000. */
  pollingRates: number[];
  /** mm */
  lodOptions: number[];
  lightingModes: LightingMode[];
  buttons: ButtonInfo[];
  totalButtons: number;
  supportsAngleSnap: boolean;
  supportsRippleControl: boolean;
  supportsMotionSync: boolean;
  /** Raw value range; null = the model has no debounce setting. */
  debounce: Range | null;
  /** ms per raw debounce unit (BK 2, inferred from the vendor label; V4 1). */
  debounceMsPerUnit: number;
  /** null = not available on this model. */
  sleepMinutes: Range | null;
  deepSleepMinutes: Range | null;
  brightness: Range;
  lightSpeed: Range;
  maxMacroEvents: number;
  supportsBle: boolean;
  supportsLighting: boolean;
  supportsMacros: boolean;
  /** false: battery over BLE is not trustworthy and is reported as null. */
  batteryOverBle: boolean;
  supportsReadBack: boolean;
}

export interface Snapshot {
  status: DeviceStatus;
  profile: Profile;
  profiles: ProfileSummary[];
  macros: Macro[];
  capabilities: Capabilities;
  /** True when the selected profile differs from what is known to be on the device. */
  pendingChanges: boolean;
  /** Groups the device has not received because an apply skipped them (unverified DPI); kept until a write succeeds. */
  pendingGroups: SkippedGroup[];
  /** Stored verdict of the DPI encoding check for this model; null before any check. */
  dpiEncoding: DpiEncoding | null;
}

/** Persisted DPI check verdict. `verified` with a `format` (a `capabilities.dpiEncoding` name), else inconclusive. */
export interface DpiEncoding {
  verified: boolean;
  format: Capabilities["dpiEncoding"] | null;
  /** RFC 3339 UTC. */
  at: string;
  /** Firmware version read with the verdict, null if unanswered. */
  firmware: number | null;
}

export type VerificationStatus = "verified" | "inconclusive" | "unsupported_link" | "timeout";

/** verify_dpi_encoding result. `format` only with "verified"; `reason` explains the evidence or the failure. */
export interface DpiVerification {
  status: VerificationStatus;
  format?: Capabilities["dpiEncoding"];
  reason?: string;
}

/** dpi-verification event: "running" when a check starts (manual or automatic), "done" with its result. */
export interface DpiVerificationEvent {
  state: "running" | "done";
  result?: DpiVerification;
}

export interface DpiStagePatch {
  index: number;
  dpi?: number;
  enabled?: boolean;
  color?: string;
}

export interface LightingPatch {
  mode?: number;
  brightness?: number;
  speed?: number;
  color?: string;
}

export interface SettingsPatch {
  dpiStages?: DpiStagePatch[];
  activeStage?: number;
  polling?: number;
  lod?: number;
  rippleControl?: boolean;
  angleSnap?: boolean;
  motionSync?: boolean;
  debounce?: number;
  sleepMinutes?: number;
  deepSleepMinutes?: number;
  lighting?: LightingPatch;
}

export type ApplyStatus = "queued" | "nothing" | "no-device" | "asleep";

export interface ApplyResult {
  status: ApplyStatus;
  packets: number;
  /** Groups left out on purpose; everything else was sent (partial success). */
  skippedGroups: SkippedGroup[];
}

/** A packet group apply did not send. Today only DPI with an encoding not verified on hardware. */
export interface SkippedGroup {
  group: "dpi";
  code: "unverified_encoding";
}

export interface SettingsResult {
  profile: Profile;
  apply: ApplyResult;
}

export interface ActionEntry {
  id: number;
  label: string;
}

export interface ActionCategory {
  id: "mouse" | "dpi" | "media" | "browser" | "function" | "shortcut";
  label: string;
  actions: ActionEntry[];
}

export interface KeyEntry {
  code: number;
  label: string;
}

export interface ModifierEntry {
  bit: number;
  label: string;
}

export interface ActionCatalog {
  categories: ActionCategory[];
  keys: KeyEntry[];
  modifiers: ModifierEntry[];
}

export type ErrorCode =
  | "invalid_input"
  | "not_found"
  | "confirm_required"
  | "last_profile"
  | "busy"
  | "io"
  | "unverified_encoding"
  | "unsupported_model"
  /** read_back: the model or the current link has no read-back. */
  | "unsupported"
  /** read_back: the device never reported the data as ready. */
  | "timeout"
  /** read_back: wrong reply header or an undecoded layout (bulk read of newer firmware). */
  | "decode_failed";

export interface ApiError {
  code: ErrorCode;
  message: string;
}

export type ProgressState = "pending" | "retry" | "ok" | "failed";

export interface ApplyProgress {
  state: ProgressState;
  packets: number;
  /** Packets not sent by design (polling rate over Bluetooth). */
  skipped: number;
  failed: number;
  attempt: number | null;
  reason: string | null;
  /** Same as ApplyResult.skippedGroups for this apply. */
  skippedGroups: SkippedGroup[];
  /** `ok` over Bluetooth: the mouse acknowledges no BLE write, so the profile stays pending. */
  unconfirmed: boolean;
}

export interface DpiStageChanged {
  /** 0-based */
  stage: number;
  dpi: number;
}

export interface DeviceEventRaw {
  event: string;
}

export interface EventMap {
  "device-status": DeviceStatus;
  "dpi-stage-changed": DpiStageChanged;
  "apply-progress": ApplyProgress;
  "device-event-raw": DeviceEventRaw;
  "dpi-verification": DpiVerificationEvent;
}

export type EventName = keyof EventMap;

/** Every command with its argument object (camelCase) and result. */
export interface CommandMap {
  get_snapshot: { args: Record<string, never>; result: Snapshot };
  list_profiles: { args: Record<string, never>; result: ProfileSummary[] };
  create_profile: { args: { name: string }; result: Snapshot };
  rename_profile: { args: { id: number; name: string }; result: Snapshot };
  delete_profile: { args: { id: number }; result: Snapshot };
  select_profile: { args: { id: number }; result: Snapshot };
  import_profile: { args: { path: string }; result: Snapshot };
  export_profile: { args: { id: number; path: string }; result: null };
  reset_profile: { args: { id: number; confirm: boolean }; result: SettingsResult };
  update_settings: { args: { patch: SettingsPatch }; result: SettingsResult };
  set_button_action: { args: { button: number; action: ActionRef }; result: SettingsResult };
  list_actions: { args: Record<string, never>; result: ActionCatalog };
  list_macros: { args: Record<string, never>; result: Macro[] };
  create_macro: { args: { input: MacroInput }; result: Macro[] };
  update_macro: { args: { id: number; input: MacroInput }; result: Macro[] };
  delete_macro: { args: { id: number }; result: Macro[] };
  apply_all: { args: Record<string, never>; result: ApplyResult };
  read_back: { args: Record<string, never>; result: ReadBack };
  verify_dpi_encoding: { args: Record<string, never>; result: DpiVerification };
}

export type CommandName = keyof CommandMap;
