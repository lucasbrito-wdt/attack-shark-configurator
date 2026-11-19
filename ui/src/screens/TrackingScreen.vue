<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import AppIcon from "../components/ui/AppIcon.vue";
import ScreenHead from "../components/ui/ScreenHead.vue";
import SegmentedControl from "../components/ui/SegmentedControl.vue";
import ToggleSwitch from "../components/ui/ToggleSwitch.vue";
import { fmt } from "../lib/styles";
import { useDevice } from "../stores/device";
import LiftOffFigure from "./tracking/LiftOffFigure.vue";

const dev = useDevice();
const { t } = useI18n();
const p = computed(() => dev.profile!);
const caps = computed(() => dev.caps!);
const ble = computed(() => dev.status?.connected && dev.status.mode === "ble");

const RATE_NAME: Record<number, "saving" | "office" | "gaming" | "esports"> = { 125: "saving", 250: "office", 500: "gaming", 1000: "esports" };
const rates = computed(() => caps.value.pollingRates.map((hz) => ({ value: hz, label: `${fmt(hz)} Hz`, sub: t(`tracking.rate.${RATE_NAME[hz] ?? "receiver4k"}`) })));
const highRates = computed(() => caps.value.pollingRates.some((hz) => hz > 1000));
const lods = computed(() => caps.value.lodOptions.map((mm) => ({ value: mm, label: `${fmt(mm)} mm` })));

const TOGGLES = [
  { key: "rippleControl", cap: "supportsRippleControl" },
  { key: "angleSnap", cap: "supportsAngleSnap" },
  { key: "motionSync", cap: "supportsMotionSync" },
] as const;
// a setting the model lacks is null in the profile and false in caps: no control at all
const toggles = computed(() => TOGGLES.flatMap((t) => (caps.value[t.cap] && p.value[t.key] != null ? [{ ...t, on: p.value[t.key] === true }] : [])));
</script>

<template>
  <div>
    <ScreenHead :title="t('nav.tracking')" :sub="t('tracking.sub')" />
    <div :class="['grid gap-8', lods.length ? 'grid-cols-[minmax(0,1fr)_260px] @max-4xl:grid-cols-1' : 'max-w-[760px] grid-cols-1']">
      <div class="divide-y divide-shell-deep rounded-panel bg-skate px-6 shadow-lift ring-1 ring-shell-deep">
        <div class="py-5">
          <div class="flex items-baseline justify-between gap-4">
            <h2 class="font-semibold">{{ t("tracking.polling") }}</h2>
            <span class="text-micro text-wheel-muted">{{ t("tracking.perSecond") }}</span>
          </div>
          <SegmentedControl :name="t('tracking.polling')" :options="rates" :value="p.polling" :disabled="ble" :class="['mt-3 flex-wrap', rates.length > 4 && '@max-4xl:[&>*]:basis-[24%]']" @select="(hz) => dev.update({ polling: hz })" />
          <p v-if="ble" class="mt-3 flex items-center gap-2 text-micro text-wheel-muted">
            <AppIcon name="ble" class="size-4" />{{ t("tracking.bleRate") }}
          </p>
          <p v-else class="mt-3 text-micro text-wheel-muted">
            {{ t("tracking.higher") }}<template v-if="highRates"> {{ t("tracking.needs4k", { hz: fmt(1000) }) }}</template>
          </p>
        </div>
        <div v-if="lods.length" class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-8 py-5">
          <div>
            <h2 class="font-semibold">{{ t("tracking.lod") }}</h2>
            <p class="mt-1 max-w-[56ch] text-wheel-muted">{{ t("tracking.lodText") }}</p>
          </div>
          <SegmentedControl :name="t('tracking.lod')" :options="lods" :value="p.lod" class="w-44" @select="(mm) => dev.update({ lod: mm })" />
        </div>
        <div v-for="tg in toggles" :key="tg.key" class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-8 py-5">
          <div>
            <h2 class="font-semibold">{{ t(`tracking.${tg.key}.title`) }}</h2>
            <p class="mt-1 max-w-[56ch] text-wheel-muted">{{ t(`tracking.${tg.key}.text`) }}</p>
          </div>
          <ToggleSwitch :on="tg.on" :label="t(`tracking.${tg.key}.title`)" @toggle="dev.update({ [tg.key]: !tg.on })" />
        </div>
      </div>
      <figure v-if="lods.length" class="self-start rounded-panel bg-skate/60 p-5 ring-1 ring-shell-deep @max-4xl:hidden" :aria-label="t('tracking.sideViewLabel')">
        <LiftOffFigure :lod="p.lod" :options="caps.lodOptions" />
        <figcaption class="mt-3 text-micro text-wheel-muted">{{ t("tracking.sideView") }}</figcaption>
      </figure>
    </div>
  </div>
</template>
