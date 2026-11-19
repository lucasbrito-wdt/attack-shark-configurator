import { defineStore } from "pinia";
import { reactive, ref } from "vue";

export type ScreenId = "overview" | "dpi" | "tracking" | "buttons" | "macros" | "lighting" | "power";

export const SCREENS: { id: ScreenId }[] = [
  { id: "overview" },
  { id: "dpi" },
  { id: "tracking" },
  { id: "buttons" },
  { id: "macros" },
  { id: "lighting" },
  { id: "power" },
];

export interface Toast {
  id: number;
  kind: "ok" | "error";
  text: string;
  /** stage color for the "changed on the mouse" toast */
  color?: string;
}

export interface ConfirmRequest {
  title: string;
  body?: string;
  ok: string;
  danger?: boolean;
  input?: { label: string; value: string; maxlength: number; validate: (v: string) => string | null };
  resolve: (v: string | boolean | null) => void;
}

const fromUrl = (): ScreenId => {
  const s = typeof location === "undefined" ? null : new URLSearchParams(location.search).get("screen");
  return SCREENS.some((x) => x.id === s) ? (s as ScreenId) : "overview";
};

export const useUi = defineStore("ui", () => {
  const screen = ref<ScreenId>(fromUrl());
  const toasts = ref<Toast[]>([]);
  /** true while a key capture or macro recording owns the keyboard */
  const keyboardBusy = ref(false);
  /** bumps when the mouse changes stage by itself: the stage light reacts */
  const stagePulse = ref(0);
  const confirmReq = ref<ConfirmRequest | null>(null);
  const selection = reactive({ stage: null as number | null, button: null as number | null, macro: null as number | null });

  let seq = 0;
  function toast(kind: Toast["kind"], text: string, color?: string) {
    const t = { id: ++seq, kind, text, color };
    toasts.value = [...toasts.value.slice(-2), t];
    setTimeout(() => (toasts.value = toasts.value.filter((x) => x.id !== t.id)), 3600);
  }

  function go(id: ScreenId) {
    screen.value = id;
  }

  function confirm(req: Omit<ConfirmRequest, "resolve">): Promise<string | boolean | null> {
    return new Promise((resolve) => (confirmReq.value = { ...req, resolve }));
  }

  return { screen, toasts, keyboardBusy, stagePulse, confirmReq, selection, toast, go, confirm };
});
