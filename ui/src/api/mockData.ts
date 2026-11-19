// Static data for the mock backend: mirrors crates/shark-core (defaults, action catalog). Capabilities come from mockModels.ts.

import type { ActionCatalog, ActionCategory, Capabilities, KeyEntry, Profile } from "./types";

import { models } from "./mockModels";

export { models };
/** Default mock model: the X6. */
export const capabilities: Capabilities = models.x6;

const cat = (id: ActionCategory["id"], label: string, actions: [number, string][]): ActionCategory => ({
  id,
  label,
  actions: actions.map(([id, label]) => ({ id, label })),
});

export const categories: ActionCategory[] = [
  cat("mouse", "Mouse", [
    [0x00, "Button Off"], [0x01, "Left Click"], [0x02, "Right Click"], [0x03, "Middle Button"], [0x04, "Forward"],
    [0x05, "Backward"], [0x09, "Double Click"], [0x0a, "Fire Button"], [0x0b, "Scroll Up"], [0x0c, "Scroll Down"],
    [0x0d, "Scroll to Left"], [0x0e, "Scroll to Right"],
  ]),
  cat("dpi", "DPI", [[0x06, "DPI Cycle"], [0x07, "DPI +"], [0x08, "DPI -"]]),
  cat("media", "Multimedia", [
    [0x40, "Media Player"], [0x41, "Play / Pause"], [0x42, "Stop"], [0x43, "Previous Track"], [0x44, "Next Track"],
    [0x45, "Volume +"], [0x46, "Volume -"], [0x47, "Mute"],
  ]),
  cat("browser", "Browser and apps", [
    [0x50, "Browser Home"], [0x51, "Browser Favorites"], [0x52, "Browser Forward"], [0x53, "Browser Backward"],
    [0x54, "Browser Stop"], [0x55, "Browser Refresh"], [0x56, "Browser Search"], [0x57, "Email"], [0x58, "Calculator"],
    [0x59, "My Computer"],
  ]),
  cat("function", "Functions", [[0x16, "Easy Aim"], [0x17, "LED Loop"]]),
  cat("shortcut", "Shortcuts", [
    [0x20, "Cut"], [0x21, "Copy"], [0x22, "Paste"], [0x23, "Open"], [0x24, "Save"], [0x25, "Find"], [0x26, "Redo"],
    [0x27, "Undo"], [0x28, "Select All"], [0x29, "Print"], [0x2a, "Close Window"], [0x2b, "Swap Windows"],
    [0x2c, "Show Desktop"], [0x2d, "Run Command"], [0x2e, "Lock PC"], [0x32, "Screen Capture"],
  ]),
];

const fixed: Record<number, string> = {
  0x28: "Enter", 0x29: "Esc", 0x2a: "Backspace", 0x2b: "Tab", 0x2c: "Space", 0x2d: "-", 0x2e: "=", 0x2f: "[", 0x30: "]",
  0x31: "\\", 0x33: ";", 0x34: "'", 0x35: "`", 0x36: ",", 0x37: ".", 0x38: "/", 0x39: "CapsLock", 0x46: "PrintScreen",
  0x47: "ScrollLock", 0x48: "Pause", 0x49: "Insert", 0x4a: "Home", 0x4b: "PageUp", 0x4c: "Delete", 0x4d: "End",
  0x4e: "PageDown", 0x4f: "Right", 0x50: "Left", 0x51: "Down", 0x52: "Up", 0xe0: "LCtrl", 0xe1: "LShift", 0xe2: "LAlt",
  0xe3: "LWin", 0xe4: "RCtrl", 0xe5: "RShift", 0xe6: "RAlt", 0xe7: "RWin", 0xf1: "Left Click", 0xf2: "Right Click",
  0xf3: "Middle Click", 0xf4: "Forward", 0xf5: "Backward",
};

export function keyLabel(code: number): string {
  if (fixed[code]) return fixed[code];
  if (code >= 0x04 && code <= 0x1d) return String.fromCharCode(65 + code - 0x04);
  if (code >= 0x1e && code <= 0x27) return "1234567890"[code - 0x1e];
  if (code >= 0x3a && code <= 0x45) return `F${code - 0x3a + 1}`;
  if (code >= 0x68 && code <= 0x73) return `F${code - 0x68 + 13}`;
  return "";
}

const keys: KeyEntry[] = [];
for (let code = 0x04; code <= 0xf5; code++) {
  const label = keyLabel(code);
  if (label) keys.push({ code, label });
}

export const catalog: ActionCatalog = {
  categories,
  keys,
  modifiers: [{ bit: 1, label: "Ctrl" }, { bit: 2, label: "Shift" }, { bit: 4, label: "Alt" }, { bit: 8, label: "Win/Cmd" }],
};

export const actionLabel = (id: number): string =>
  categories.flatMap((c) => c.actions).find((a) => a.id === id)?.label ?? "Left Click";

const stageColors = ["#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF", "#FF00FF", "#FF4000", "#FFFFFF"];
const units = [16, 32, 48, 64, 100, 520, 0, 0];

export function defaultProfile(id: number, name: string): Profile {
  const preset: Record<number, number> = { 0: 1, 1: 2, 2: 3, 6: 5, 7: 4 };
  return {
    id,
    name,
    dpiStages: units.map((u, index) => ({ index, dpi: u * 50, enabled: index < 6, color: stageColors[index] })),
    activeStage: 1,
    polling: 1000,
    lod: 1,
    rippleControl: false,
    angleSnap: false,
    motionSync: false,
    debounce: 4,
    sleepMinutes: 1,
    deepSleepMinutes: 10,
    lighting: { mode: 0, brightness: 8, speed: 6, color: "#0000FF" },
    buttons: Array.from({ length: 18 }, (_, button) => {
      const actionId = preset[button] ?? 0;
      return { button, action: { kind: "preset", id: actionId }, label: actionLabel(actionId) };
    }),
  };
}
