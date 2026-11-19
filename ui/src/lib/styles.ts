import { currentLocale } from "../i18n";

// Shared utility strings for the control family (one source, so every button has the same five states).

export const focusRing = "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-led-deep";
export const btnBase = `inline-flex items-center justify-center gap-2 rounded-key font-medium transition-[background-color,box-shadow,transform,color] duration-150 ease-actuation active:scale-[0.97] disabled:cursor-not-allowed disabled:opacity-45 disabled:active:scale-100 ${focusRing}`;
export const btnPrimary = `${btnBase} h-10 px-5 bg-wheel text-skate shadow-lift hover:bg-wheel/88 active:shadow-press`;
export const btnSecondary = `${btnBase} h-10 px-4 bg-skate text-wheel shadow-lift ring-1 ring-shell-deep hover:ring-wheel-muted/50 active:shadow-press`;
export const btnGhost = `${btnBase} h-9 px-3 text-wheel-muted hover:bg-shell hover:text-wheel`;
export const btnIcon = `${btnBase} size-9 text-wheel-muted hover:bg-shell hover:text-wheel`;
export const btnDanger = `${btnBase} h-10 px-5 bg-sensor text-skate shadow-lift hover:bg-sensor/90`;
export const panel = "rounded-panel bg-skate shadow-lift ring-1 ring-shell-deep";
export const fieldNumber = "num h-9 rounded-key bg-skate px-2 text-right text-wheel ring-1 ring-shell-deep focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-led-deep";

/** Number in the active UI language: 1,600 / 1.600. Reactive when read during render. */
export const fmt = (n: number): string => n.toLocaleString(currentLocale());
