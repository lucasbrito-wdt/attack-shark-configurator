// vue-i18n (Composition API). en is the source of truth; pt-BR and es are typed against it.
import { createI18n } from "vue-i18n";
import { isTauri } from "./api/client";
import en, { type MessageSchema } from "./locales/en";
import es from "./locales/es";
import ptBR from "./locales/pt-BR";

declare module "vue-i18n" {
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type
  export interface DefineLocaleMessage extends MessageSchema {}
}

/** Shown in their own language in the selector. */
export const LOCALES = [
  { id: "pt-BR", name: "Português" },
  { id: "en", name: "English" },
  { id: "es", name: "Español" },
] as const;
export type Locale = (typeof LOCALES)[number]["id"];

const KEY = "x3.locale";

/** Saved choice first, then the system language (pt* -> pt-BR, es* -> es, else en). */
export function detectLocale(saved: string | null, system: string): Locale {
  if (LOCALES.some((l) => l.id === saved)) return saved as Locale;
  const s = system.toLowerCase();
  return s.startsWith("pt") ? "pt-BR" : s.startsWith("es") ? "es" : "en";
}

function saved(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

export const i18n = createI18n({
  legacy: false,
  locale: detectLocale(saved(), (typeof navigator === "undefined" ? undefined : navigator.language) ?? "en"),
  fallbackLocale: "en",
  messages: { en, "pt-BR": ptBR, es },
});

export const t = i18n.global.t;
export const currentLocale = (): Locale => i18n.global.locale.value as Locale;

/** Switch the UI language; `persist` only for an explicit choice, so the system language keeps working otherwise. */
export async function setLocale(l: Locale, persist = true): Promise<void> {
  i18n.global.locale.value = l;
  document.documentElement.lang = l;
  if (persist) {
    try {
      localStorage.setItem(KEY, l);
    } catch {
      // private mode or storage off: the choice lasts for this session
    }
  }
  if (!isTauri()) return;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("set_locale", { locale: l });
  } catch {
    // the tray menu keeps its previous language; the window is already switched
  }
}
