<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import BatteryMeter from "../components/ui/BatteryMeter.vue";
import RailInput from "../components/ui/RailInput.vue";
import ScreenHead from "../components/ui/ScreenHead.vue";
import { useBattery } from "../composables/useBattery";
import { RELEASES_URL, openReleases, updatesSupported, useUpdater } from "../composables/useUpdater";
import { btnSecondary } from "../lib/styles";
import { useDevice } from "../stores/device";

const dev = useDevice();
const { t } = useI18n();
const updater = useUpdater();
const upd = updater.state;
const battery = useBattery();
const p = computed(() => dev.profile!);
const caps = computed(() => dev.caps!);
const isWord = computed(() => battery.value.level == null);

const TIMERS = ["sleepMinutes", "deepSleepMinutes"] as const;
// a timer the model lacks has a null range and a null profile value: no control, no segment
const timers = computed(() =>
  TIMERS.flatMap((key) => {
    const range = caps.value[key];
    const min = p.value[key];
    return range && min != null ? [{ key, title: t(`power.${key}.title`), note: t(`power.${key}.note`), state: t(`power.${key}.state`), range, min }] : [];
  }),
);
const deepBeforeSleep = computed(() => p.value.sleepMinutes != null && p.value.deepSleepMinutes != null && p.value.deepSleepMinutes < p.value.sleepMinutes);

/** Awake, then each rest state from its timer on, on a line as long as the longest timer allows. */
const rest = computed(() => {
  const span = Math.max(...timers.value.map((t) => t.range.max));
  const stops = [...timers.value].sort((a, b) => a.min - b.min);
  return {
    span,
    segs: [
      { grow: stops[0]?.min ?? span, label: t("power.awake"), cls: "bg-wheel" },
      ...stops.map((s, i) => ({ grow: (stops[i + 1]?.min ?? span) - s.min, label: t("power.from", { state: s.state, min: s.min }), cls: i === stops.length - 1 ? "bg-shell-deep" : "bg-wheel-muted/50" })),
    ],
    sr: t("power.restSr", { states: stops.map((s) => t("power.fromLong", { state: s.state.toLocaleLowerCase(), min: s.min }, s.min)).join(", ") }),
  };
});
</script>

<template>
  <div>
    <ScreenHead :title="t('nav.power')" :sub="timers.length ? t('power.sub') : t('power.subNoTimers', { model: caps.modelName })" />
    <div class="grid grid-cols-[5fr_6fr] gap-8 @max-4xl:grid-cols-1">
      <section :aria-label="t('battery.title')" :class="['rounded-panel bg-skate', !timers.length && 'col-span-full']" class="p-6 shadow-lift ring-1 ring-shell-deep">
        <p class="text-wheel-muted">{{ t("battery.title") }}</p>
        <p v-if="isWord" :class="['wide mt-2 text-display font-[260]', !battery.charging && 'text-wheel-muted']">{{ battery.big }}</p>
        <p v-else :class="['num mt-2 text-numeral font-[260] tracking-[-0.03em]', battery.low && 'text-sensor', battery.stale && 'opacity-55']">{{ battery.big }}</p>
        <BatteryMeter :class="['mt-5 h-4', battery.stale && 'opacity-45']" :level="battery.level" :charging="battery.charging" />
        <p class="mt-4 max-w-[48ch] text-wheel-muted">{{ battery.small }}</p>
      </section>
      <section v-if="timers.length" :aria-label="t('power.timers')" class="divide-y divide-shell-deep rounded-panel bg-skate px-6 shadow-lift ring-1 ring-shell-deep">
        <div v-for="tm in timers" :key="tm.key" class="py-5">
          <div class="flex items-baseline justify-between">
            <h2 class="font-semibold">{{ tm.title }}</h2>
            <span class="num text-title">{{ t("power.min", { n: tm.min }) }}</span>
          </div>
          <RailInput :min="tm.range.min" :max="tm.range.max" :value="tm.min" :label="t('power.minutesOf', { title: tm.title })" @input="(v) => dev.update({ [tm.key]: v }, true)" @commit="() => dev.flush()" />
          <p class="text-micro text-wheel-muted">{{ tm.note }}</p>
          <p v-if="tm.key === 'deepSleepMinutes' && deepBeforeSleep" class="mt-2 text-micro font-semibold text-sensor" role="alert">{{ t("power.deepBeforeSleep") }}</p>
        </div>
      </section>
      <section v-if="timers.length" :aria-label="t('power.leaveAria')" class="col-span-full rounded-panel bg-skate/60 p-6 ring-1 ring-shell-deep">
        <h2 class="font-semibold">{{ t("power.leave") }}</h2>
        <div class="mt-4">
          <div class="flex gap-1" aria-hidden="true">
            <div v-for="s in rest.segs" :key="s.cls" class="flex min-w-16 flex-col gap-2" :style="{ flex: Math.max(s.grow, 0.001) }">
              <div :class="['h-3 rounded-full transition-[flex-grow] duration-300 ease-actuation', s.cls]" />
              <span class="truncate text-micro text-wheel-muted">{{ s.label }}</span>
            </div>
          </div>
          <p class="sr-only">{{ rest.sr }}</p>
          <div class="num mt-1 flex justify-between text-micro text-wheel-muted"><span>0</span><span>{{ t("power.min", { n: rest.span }) }}</span></div>
        </div>
      </section>
    </div>
    <footer v-if="updatesSupported()" class="mt-8 flex flex-wrap items-center gap-x-4 gap-y-2 border-t border-shell-deep pt-5 text-micro text-wheel-muted">
      <span class="num">{{ t("update.version", { version: upd.currentVersion }) }}</span>
      <span aria-live="polite" class="min-w-0 flex-1">
        <template v-if="upd.phase === 'checking'">{{ t("update.checking") }}</template>
        <template v-else-if="upd.phase === 'upToDate'">{{ t("update.upToDate") }}</template>
        <template v-else-if="upd.phase !== 'idle'">{{ t("update.title", { version: upd.version }) }}</template>
        <span v-else-if="upd.error && upd.manual" class="font-semibold text-wheel">{{ t(`update.error.${upd.error}`) }}</span>
        <a v-if="upd.phase === 'idle' && upd.error === 'package'" :href="RELEASES_URL" class="ml-1 text-led-deep underline" @click.prevent="openReleases">{{ t("update.releasesLink") }}</a>
      </span>
      <button v-if="upd.phase === 'available' || upd.phase === 'downloading' || upd.phase === 'installing'" type="button" :class="btnSecondary" @click="upd.open = true">{{ t("update.see") }}</button>
      <button v-else type="button" :class="btnSecondary" :disabled="upd.phase === 'checking'" @click="updater.check(true)">{{ upd.error && upd.manual ? t("update.retry") : t("update.check") }}</button>
    </footer>
  </div>
</template>
