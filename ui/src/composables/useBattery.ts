// Battery copy for every state the contract can report. No number is ever invented.
import { computed, ref } from "vue";
import type { LastKnownBattery } from "../api/client";
import { currentLocale, t } from "../i18n";
import { useDevice } from "../stores/device";

export interface BatteryCopy {
  /** big line: a percentage or a word */
  big: string;
  small: string;
  level: number | null;
  charging: boolean;
  low: boolean;
  /** level is the persisted last reading, not a live one */
  stale: boolean;
}

/** Wall clock for relative ages; one shared 30 s tick keeps "3 h ago" honest without a timer per component. */
const now = ref(Date.now());
let ticking = false;
function startClock() {
  if (ticking || typeof window === "undefined") return;
  ticking = true;
  setInterval(() => (now.value = Date.now()), 30000);
}

/** "80%" / "80 %" in the active language. */
export const percent = (level: number, locale = currentLocale()): string => new Intl.NumberFormat(locale, { style: "percent" }).format(level / 100);

/** "just now", "12m ago", "há 7 h", "hace 2 d" */
export function ago(at: string, now = Date.now(), locale = currentLocale()): string {
  const min = Math.max(0, Math.round((now - Date.parse(at)) / 60000));
  if (min < 1) return t("battery.justNow");
  const rtf = new Intl.RelativeTimeFormat(locale, { style: "narrow", numeric: "always" });
  if (min < 60) return rtf.format(-min, "minute");
  if (min < 1440) return rtf.format(-Math.round(min / 60), "hour");
  return rtf.format(-Math.round(min / 1440), "day");
}

/** " Last seen at 70% through the 2.4G receiver, 2h ago." */
export function lastSeen(k: LastKnownBattery | null, now = Date.now()): string {
  return k ? ` ${t("battery.lastSeen", { level: percent(k.level), via: t(`battery.through.${k.link}`), age: ago(k.at, now) })}` : "";
}

export function useBattery() {
  const dev = useDevice();
  startClock();
  return computed<BatteryCopy>(() => {
    const st = dev.status;
    const none = { level: null, charging: false, low: false, stale: false };
    if (!st) return { ...none, big: t("battery.unknown"), small: t("battery.noMouse") };
    const b = st.battery;
    const last = lastSeen(b.lastKnown, now.value);
    const k = b.lastKnown;
    // no live level: fall back to the last reading, marked stale with "~"; the word only when there is none
    const prior = (word: string) => (k ? { ...none, level: k.level, low: k.level <= 20, stale: true, big: `~${percent(k.level)}` } : { ...none, big: word });
    if (!st.connected) return { ...prior(t("battery.unknown")), small: t("battery.noMouse") + last };
    if (b.reason === "not-reported-over-ble") return { ...prior(t("battery.notReported")), small: t("battery.onlyReceiverOrCable") + last };
    if (b.charging)
      return b.level != null
        ? { ...none, charging: true, level: b.level, big: percent(b.level), small: t("battery.chargingCable") }
        : { ...prior(t("battery.charging")), low: false, charging: true, small: t("battery.noLevelWhileCharging") + last };
    if (!st.awake)
      return b.level != null
        ? { ...none, level: b.level, big: percent(b.level), small: t("battery.beforeSleepLong") }
        : { ...prior(t("battery.unknown")), small: t("battery.asleepWaits") + last };
    if (b.level == null) return { ...prior(t("battery.unknown")), small: t("battery.waiting") + last };
    const low = b.level <= 20;
    const from = b.source === "ble-bas" ? t("battery.fromBas") : t("battery.fromMouse");
    return { ...none, level: b.level, low, big: percent(b.level), small: low ? `${t("battery.low")} ${from}` : from };
  });
}

export interface BatteryBadge {
  /** "80%" or "--" */
  value: string;
  /** "via Bluetooth", "via 2.4G receiver, 3h ago", or a short reason */
  detail: string;
  /** the value is the last reading, not a live one */
  stale: boolean;
  low: boolean;
  charging: boolean;
  level: number | null;
  /** full sentence for screen readers */
  label: string;
  /** explains a stale or missing value; null when live */
  tip: string | null;
}

/** The always-visible indicator: live level, else the persisted last reading marked stale, else "--". */
export function useBatteryBadge() {
  const dev = useDevice();
  const copy = useBattery();
  return computed<BatteryBadge>(() => {
    const st = dev.status;
    const b = st?.battery;
    const live = st?.connected && b?.level != null ? b.level : null;
    if (live != null) {
      const charging = b?.charging === true;
      const via = t(`battery.via.${st?.mode === "ble" ? "ble" : st?.mode === "wired" ? "usb" : "dongle"}`);
      const detail = charging ? t("battery.charging") : st?.awake ? via : t("battery.beforeSleep");
      const label = charging
        ? t("battery.label.charging", { level: percent(live) })
        : st?.awake
          ? t("battery.label.live", { level: percent(live), via })
          : t("battery.label.asleep", { level: percent(live) });
      return { value: percent(live), detail, stale: false, low: live <= 20 && !charging, charging, level: live, label, tip: null };
    }
    const k = b?.lastKnown;
    if (k) {
      const from = t("battery.lastFrom", { source: t(`battery.source.${k.link}`) });
      const tip = !st?.connected
        ? `${t("battery.noMouse")} ${from}`
        : b?.charging
          ? `${t("battery.chargingNoLevel")} ${from}`
          : b?.reason === "not-reported-over-ble"
            ? t("battery.notOverBle")
            : !st.awake
              ? `${t("battery.asleep")} ${from}`
              : `${t("battery.waitingShort")}. ${from}`;
      const detail = `${t(`battery.via.${k.link}`)}, ${ago(k.at, now.value)}`;
      const label = t("battery.label.stale", { level: percent(k.level), detail, tip });
      return { value: `~${percent(k.level)}`, detail, stale: true, low: k.level <= 20 && !b?.charging, charging: b?.charging === true, level: k.level, label, tip };
    }
    // the sidebar has room for a few words; the full reason goes in the tooltip and the label
    const short = !st?.connected
      ? t("battery.short.noMouse")
      : b?.reason === "not-reported-over-ble"
        ? t("battery.short.notOverBle")
        : b?.charging
          ? t("battery.charging")
          : !st.awake
            ? t("battery.short.asleep")
            : t("battery.waitingShort");
    return { value: "--", detail: short, stale: false, low: false, charging: false, level: null, label: t("battery.label.unknown", { reason: copy.value.small }), tip: copy.value.small };
  });
}
