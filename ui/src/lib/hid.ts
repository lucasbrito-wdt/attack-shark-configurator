// KeyboardEvent.code -> HID keyboard usage (USB HID Usage Tables, page 0x07), which is what the
// mouse stores for shortcuts and macro events. Labels come from the backend catalog.

export const IS_MAC = typeof navigator !== "undefined" && /Mac/i.test(navigator.platform || navigator.userAgent);

const NAMED: Record<string, number> = {
  Enter: 0x28, Escape: 0x29, Backspace: 0x2a, Tab: 0x2b, Space: 0x2c, Minus: 0x2d, Equal: 0x2e, BracketLeft: 0x2f,
  BracketRight: 0x30, Backslash: 0x31, Semicolon: 0x33, Quote: 0x34, Backquote: 0x35, Comma: 0x36, Period: 0x37,
  Slash: 0x38, CapsLock: 0x39, PrintScreen: 0x46, ScrollLock: 0x47, Pause: 0x48, Insert: 0x49, Home: 0x4a,
  PageUp: 0x4b, Delete: 0x4c, End: 0x4d, PageDown: 0x4e, ArrowRight: 0x4f, ArrowLeft: 0x50, ArrowDown: 0x51,
  ArrowUp: 0x52, NumLock: 0x53, NumpadDivide: 0x54, NumpadMultiply: 0x55, NumpadSubtract: 0x56, NumpadAdd: 0x57,
  NumpadEnter: 0x58, NumpadDecimal: 0x63, IntlBackslash: 0x64, ContextMenu: 0x65,
  ControlLeft: 0xe0, ShiftLeft: 0xe1, AltLeft: 0xe2, MetaLeft: 0xe3, ControlRight: 0xe4, ShiftRight: 0xe5,
  AltRight: 0xe6, MetaRight: 0xe7,
};

export function hidFromCode(code: string): number | null {
  if (code in NAMED) return NAMED[code];
  let m = /^Key([A-Z])$/.exec(code);
  if (m) return 0x04 + m[1].charCodeAt(0) - 65;
  m = /^Digit(\d)$/.exec(code);
  if (m) return m[1] === "0" ? 0x27 : 0x1e + Number(m[1]) - 1;
  m = /^F(\d{1,2})$/.exec(code);
  if (m) {
    const n = Number(m[1]);
    if (n >= 1 && n <= 12) return 0x3a + n - 1;
    if (n >= 13 && n <= 24) return 0x68 + n - 13;
  }
  m = /^Numpad(\d)$/.exec(code);
  if (m) return m[1] === "0" ? 0x62 : 0x59 + Number(m[1]) - 1;
  return null;
}

export const isModifierCode = (code: string): boolean => /^(Control|Shift|Alt|Meta)(Left|Right)$/.test(code);

/** Modifier bits as the catalog defines them: 1 Ctrl, 2 Shift, 4 Alt, 8 Win/Cmd. */
export function modsFromEvent(e: KeyboardEvent): number {
  return (e.ctrlKey ? 1 : 0) | (e.shiftKey ? 2 : 0) | (e.altKey ? 4 : 0) | (e.metaKey ? 8 : 0);
}

const MAC_LABEL: Record<string, string> = { LWin: "Cmd", RWin: "Right Cmd", LAlt: "Option", RAlt: "Right Option" };
const PLAIN_LABEL: Record<string, string> = { LCtrl: "Ctrl", LShift: "Shift", LAlt: "Alt", LWin: "Win", RCtrl: "Right Ctrl", RShift: "Right Shift", RAlt: "Right Alt", RWin: "Right Win" };

/** Display label for a catalog key label, using the names printed on this computer's keyboard. */
export function friendlyKey(label: string): string {
  if (IS_MAC && MAC_LABEL[label]) return MAC_LABEL[label];
  return PLAIN_LABEL[label] ?? label;
}

export function modLabels(mods: number): string[] {
  const out: string[] = [];
  if (mods & 1) out.push("Ctrl");
  if (mods & 2) out.push("Shift");
  if (mods & 4) out.push(IS_MAC ? "Option" : "Alt");
  if (mods & 8) out.push(IS_MAC ? "Cmd" : "Win");
  return out;
}

export const MOD_KEY = IS_MAC ? "Cmd" : "Ctrl";
