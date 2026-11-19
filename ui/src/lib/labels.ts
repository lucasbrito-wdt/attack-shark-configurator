// Translated names for labels that come from the backend; unknown ids keep the backend label.
import type { ButtonBinding, ButtonInfo } from "../api/client";
import { i18n, t } from "../i18n";

const known = (key: string) => i18n.global.te(key, "en");

/** Preset action by catalog id. */
export const actionName = (id: number, fallback: string): string => (known(`action.a${id}`) ? t(`action.a${id}`) : fallback);

/** Action category tab by category id. */
export const categoryName = (id: string, fallback: string): string => (known(`action.category.${id}`) ? t(`action.category.${id}`) : fallback);

/** A mouse button by its physical position. */
export const buttonName = (b: ButtonInfo): string => (known(`buttons.name.${b.position}`) ? t(`buttons.name.${b.position}`) : b.label);

/** What a button is bound to: preset by id, key combination and macro name kept as they are. */
export function bindingName(b: ButtonBinding): string {
  const a = b.action;
  if (a.kind === "preset") return actionName(a.id, b.label);
  const rest = b.label.includes(": ") ? b.label.slice(b.label.indexOf(": ") + 2) : b.label;
  return a.kind === "shortcut" ? t("buttons.boundShortcut", { combo: rest }) : t("buttons.boundMacro", { name: a.name ?? rest });
}

/** Lighting mode by its vendor label ("Rainbow Wave" -> lighting.mode.rainbow_wave). */
export function modeName(label: string): string {
  const key = `lighting.mode.${label.toLowerCase().replace(/[^a-z0-9]+/g, "_")}`;
  return known(key) ? t(key) : label;
}
