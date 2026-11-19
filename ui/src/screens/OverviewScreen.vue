<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import MouseSlot from "../components/stage/MouseSlot.vue";
import AppIcon, { type IconName } from "../components/ui/AppIcon.vue";
import BatteryMeter from "../components/ui/BatteryMeter.vue";
import { useBattery } from "../composables/useBattery";
import { btnSecondary, fmt, focusRing } from "../lib/styles";
import { useDevice } from "../stores/device";
import { useUi } from "../stores/ui";

const dev = useDevice();
const battery = useBattery();
const ui = useUi();
const { t } = useI18n();

async function readFromMouse() {
  const ok = await ui.confirm({
    title: t("overview.readTitle"),
    body: t("overview.readBody"),
    ok: t("overview.read"),
  });
  if (ok) await dev.readBack();
}

const p = computed(() => dev.profile!);
const mode = computed(() => (dev.status?.connected ? dev.status.mode : "none"));
const stages = computed(() => p.value.dpiStages.slice(0, dev.caps?.maxDpiStages ?? 8).filter((s) => s.enabled && s.dpi > 0));
const title = computed(() => (dev.linked ? `ATTACK SHARK ${dev.model?.name ?? ""}`.trim() : t("overview.noMouse")));
const sub = computed(() => {
  if (!dev.linked) return t("overview.plugIn");
  if (!dev.awake) return t("dock.asleep");
  if (mode.value === "ble") return dev.status?.model ? t("overview.bleAs", { name: dev.status.model }) : t("overview.ble");
  return mode.value === "dongle" ? t("overview.dongle") : t("overview.wired");
});
const linkNote = computed(() => {
  if (mode.value === "ble") return t("overview.blePolling");
  if (mode.value === "none") return t("overview.changesStay");
  return t("overview.polling", { hz: fmt(p.value.polling) });
});
const sensorNote = computed(() => {
  const i = stages.value.findIndex((s) => s.index === p.value.activeStage);
  const at = { stage: i + 1, total: stages.value.length, lod: fmt(p.value.lod) };
  return dev.caps?.lodOptions.length ? t("overview.stageOfLod", at) : t("overview.stageOf", at);
});
const rows = computed<{ icon: IconName; label: string; value: string; note: string; meter?: boolean; num?: boolean }[]>(() => [
  { icon: mode.value, label: t("overview.link"), value: t(`link.long.${mode.value}`), note: linkNote.value },
  { icon: "power", label: t("battery.title"), value: battery.value.big, note: battery.value.small, meter: true },
  { icon: "tracking", label: t("overview.sensor"), value: fmt(dev.activeStage?.dpi ?? 0), note: sensorNote.value, num: true },
]);

function choose(index: number) {
  dev.update({ activeStage: index });
}

function onStageKey(e: KeyboardEvent) {
  const i = stages.value.findIndex((s) => s.index === p.value.activeStage);
  const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
  if (!d) return;
  e.preventDefault();
  choose(stages.value[(i + d + stages.value.length) % stages.value.length].index);
  const g = (e.currentTarget as HTMLElement).closest('[role="radiogroup"]');
  requestAnimationFrame(() => (g?.querySelector('[aria-checked="true"]') as HTMLElement | null)?.focus());
}
</script>

<template>
  <div class="grid h-full min-h-[520px] grid-cols-[5fr_6fr] items-center gap-6 @max-4xl:grid-cols-[4fr_6fr]">
    <div class="flex h-full items-center justify-center">
      <MouseSlot class="h-[min(560px,calc(100dvh-200px))]" />
    </div>
    <section class="max-w-[480px]">
      <h1 class="wide text-display font-[260] tracking-[-0.02em]">{{ title }}</h1>
      <p class="mt-3 max-w-[44ch] text-lead text-wheel-muted">{{ sub }}</p>
      <div class="mt-6 divide-y divide-shell-deep border-y border-shell-deep">
        <div v-for="r in rows" :key="r.label" class="grid grid-cols-[40px_1fr] gap-4 py-4">
          <span class="grid size-10 place-items-center rounded-full bg-skate text-wheel shadow-lift ring-1 ring-shell-deep"><AppIcon :name="r.icon" class="size-[18px]" /></span>
          <div class="min-w-0">
            <div class="flex items-baseline justify-between gap-4">
              <span class="text-wheel-muted">{{ r.label }}</span>
              <span :class="['wide text-title font-[450]', r.meter && battery.low && 'text-sensor', r.meter && battery.stale && 'opacity-55']"><span :class="r.num && 'num'">{{ r.value }}</span><template v-if="r.num"> DPI</template></span>
            </div>
            <p class="mt-1 text-micro text-wheel-muted">{{ r.note }}</p>
            <BatteryMeter v-if="r.meter" :class="['mt-2 h-2', battery.stale && 'opacity-45']" :level="battery.level" :charging="battery.charging && battery.level == null" />
          </div>
        </div>
      </div>
      <div class="mt-5">
        <p class="mb-2 text-micro text-wheel-muted">{{ t("overview.activeStageHint") }}</p>
        <div role="radiogroup" :aria-label="t('overview.activeStage')" class="flex flex-wrap gap-1">
          <button
            v-for="s in stages"
            :key="s.index"
            type="button"
            role="radio"
            :aria-checked="s.index === p.activeStage"
            :tabindex="s.index === p.activeStage ? 0 : -1"
            :style="{ '--stage': s.color }"
            :class="[
              'flex h-8 items-center gap-2 rounded-full px-2 transition-[background-color,box-shadow] duration-200 ease-actuation active:scale-[0.97]',
              s.index === p.activeStage ? 'bg-skate shadow-lift ring-1 ring-shell-deep' : 'hover:bg-skate/60',
              focusRing,
            ]"
            @click="choose(s.index)"
            @keydown="onStageKey"
          >
            <span :class="['lens size-3 rounded-full', s.index === p.activeStage && 'lens-glow']" />
            <span class="num text-micro font-semibold">{{ fmt(s.dpi) }}</span>
          </button>
        </div>
      </div>
      <div v-if="dev.model?.readBack" class="mt-5 flex items-center gap-3">
        <button type="button" :class="btnSecondary" :disabled="!dev.canReadBack || dev.reading" aria-describedby="read-note" @click="readFromMouse">
          <span v-if="dev.reading" class="spin size-4 rounded-full border-2 border-shell-deep border-t-wheel" /><AppIcon v-else name="read" class="size-4" />
          {{ dev.reading ? t("overview.reading") : t("overview.read") }}
        </button>
        <span id="read-note" class="max-w-[40ch] text-micro text-wheel-muted" aria-live="polite">{{
          dev.reading ? t("overview.readTakes") : dev.canReadBack ? t("overview.readNote") : t("overview.readNeedsLink")
        }}</span>
      </div>
    </section>
  </div>
</template>
