// Thin typed wrapper over @tauri-apps/api. Outside Tauri (plain browser) it talks to the in-memory mock.

import type {
  ActionCatalog,
  ActionRef,
  ApplyResult,
  CommandMap,
  CommandName,
  DpiVerification,
  EventMap,
  EventName,
  Macro,
  MacroInput,
  ProfileSummary,
  ReadBack,
  SettingsPatch,
  SettingsResult,
  Snapshot,
} from "./types";
import { MockBackend } from "./mock";

export * from "./types";

export const isTauri = (): boolean => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const mock = new MockBackend();

async function call<K extends CommandName>(cmd: K, args: CommandMap[K]["args"] = {} as never): Promise<CommandMap[K]["result"]> {
  if (!isTauri()) return mock.invoke(cmd, args);
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke(cmd, args as Record<string, unknown>);
}

/** Subscribe to a backend event. Returns an unsubscribe function. */
export async function on<E extends EventName>(event: E, handler: (payload: EventMap[E]) => void): Promise<() => void> {
  if (!isTauri()) return mock.listen(event, handler);
  const { listen } = await import("@tauri-apps/api/event");
  return listen<EventMap[E]>(event, (e) => handler(e.payload));
}

const JSON_FILTER = [{ name: "Profile", extensions: ["json"] }];

/** Native open dialog (mock: a fake path). null when cancelled. */
export async function pickOpenPath(): Promise<string | null> {
  if (!isTauri()) return "/mock/profile.json";
  const { open } = await import("@tauri-apps/plugin-dialog");
  const path = await open({ multiple: false, directory: false, filters: JSON_FILTER });
  return typeof path === "string" ? path : null;
}

/** Native save dialog (mock: a fake path). null when cancelled. */
export async function pickSavePath(defaultName: string): Promise<string | null> {
  if (!isTauri()) return `/mock/${defaultName}.json`;
  const { save } = await import("@tauri-apps/plugin-dialog");
  return save({ defaultPath: `${defaultName}.json`, filters: JSON_FILTER });
}

export const api = {
  getSnapshot: (): Promise<Snapshot> => call("get_snapshot"),
  listProfiles: (): Promise<ProfileSummary[]> => call("list_profiles"),
  createProfile: (name: string): Promise<Snapshot> => call("create_profile", { name }),
  renameProfile: (id: number, name: string): Promise<Snapshot> => call("rename_profile", { id, name }),
  deleteProfile: (id: number): Promise<Snapshot> => call("delete_profile", { id }),
  selectProfile: (id: number): Promise<Snapshot> => call("select_profile", { id }),
  importProfile: (path: string): Promise<Snapshot> => call("import_profile", { path }),
  exportProfile: (id: number, path: string): Promise<null> => call("export_profile", { id, path }),
  /** Sends the risky 0x0C resync to the device: the caller must have asked the user first. */
  resetProfile: (id: number, confirm: boolean): Promise<SettingsResult> => call("reset_profile", { id, confirm }),
  updateSettings: (patch: SettingsPatch): Promise<SettingsResult> => call("update_settings", { patch }),
  setButtonAction: (button: number, action: ActionRef): Promise<SettingsResult> => call("set_button_action", { button, action }),
  listActions: (): Promise<ActionCatalog> => call("list_actions"),
  listMacros: (): Promise<Macro[]> => call("list_macros"),
  createMacro: (input: MacroInput): Promise<Macro[]> => call("create_macro", { input }),
  updateMacro: (id: number, input: MacroInput): Promise<Macro[]> => call("update_macro", { id, input }),
  deleteMacro: (id: number): Promise<Macro[]> => call("delete_macro", { id }),
  applyAll: (): Promise<ApplyResult> => call("apply_all"),
  /** Reads the device's current settings (read-only). Errors: unsupported, timeout, decode_failed, busy. */
  readBack: (): Promise<ReadBack> => call("read_back"),
  /** Read-only DPI encoding check; the verdict is stored and enables DPI writes when verified. Errors: busy, unsupported_model. */
  verifyDpiEncoding: (): Promise<DpiVerification> => call("verify_dpi_encoding"),
};
