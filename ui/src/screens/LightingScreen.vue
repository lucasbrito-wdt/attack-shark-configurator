<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { LightingMode } from "../api/client";
import MouseSlot from "../components/stage/MouseSlot.vue";
import AppIcon from "../components/ui/AppIcon.vue";
import RailInput from "../components/ui/RailInput.vue";
import ScreenHead from "../components/ui/ScreenHead.vue";
import { modeName } from "../lib/labels";
import { focusRing } from "../lib/styles";
import { useDevice } from "../stores/device";

const dev = useDevice();
const { t } = useI18n();
const p = computed(() => dev.profile!);
const caps = computed(() => dev.caps!);
const l = computed(() => p.value.lighting);
const COLORS = ["#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF", "#FF00FF", "#FF4000", "#FFFFFF"];
const SPEED_KEYS = ["slowest", "slow", "medium", "fast", "fastest"] as const;

// brightness comes from the contract; color and speed are always sent and their use is not decoded (null), so they stay usable
function flags(m: LightingMode | undefined) {
  const off = !m || /\boff\b/i.test(m.label);
  return { off, color: !off && m.usesColor !== false, brightness: !off && m.usesBrightness, speed: !off && m.usesSpeed !== false };
}
const mode = computed(() => caps.value.lightingModes.find((m) => m.id === l.value.mode));
const f = computed(() => flags(mode.value));
const modeLabel = computed(() => (mode.value ? modeName(mode.value.label) : ""));
const sub = computed(() =>
  caps.value.supportsLighting
    ? t("lighting.sub", { model: caps.value.modelName })
    : t("lighting.subIndicator", { model: caps.value.modelName }),
);
const speedLabel = computed(() => {
  const r = caps.value.lightSpeed;
  return r.max - r.min === SPEED_KEYS.length - 1 ? t(`lighting.speedName.${SPEED_KEYS[l.value.speed - r.min]}`) : t("lighting.of", { value: l.value.speed, max: r.max });
});

function onModesKey(e: KeyboardEvent) {
  const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
  if (!d) return;
  e.preventDefault();
  const list = caps.value.lightingModes;
  const i = list.findIndex((m) => m.id === l.value.mode);
  dev.update({ lighting: { mode: list[(i + d + list.length) % list.length].id } });
  const g = e.currentTarget as HTMLElement;
  requestAnimationFrame(() => (g.querySelector('[aria-checked="true"]') as HTMLElement | null)?.focus());
}
</script>

<template>
  <div>
    <ScreenHead :title="t('nav.lighting')" :sub="caps.lightingModes.length ? sub : undefined" />
    <div v-if="!caps.lightingModes.length" class="grid max-w-[640px] gap-3 rounded-panel bg-skate/60 p-8 ring-1 ring-shell-deep">
      <AppIcon name="lighting" class="size-6 text-wheel-muted" />
      <h2 class="font-semibold">{{ t("lighting.noneTitle") }}</h2>
      <p class="text-wheel-muted">{{ t("lighting.noneBody", { model: caps.modelName }) }}</p>
    </div>
    <div v-else class="grid grid-cols-[220px_minmax(0,1fr)] gap-8 @max-4xl:grid-cols-[160px_minmax(0,1fr)]">
      <div><MouseSlot class="w-full" /></div>
      <div class="flex flex-col gap-4">
        <div role="radiogroup" :aria-label="t('lighting.mode.label')" class="grid grid-cols-4 gap-2 @max-4xl:grid-cols-3" @keydown="onModesKey">
          <button
            v-for="m in caps.lightingModes"
            :key="m.id"
            type="button"
            role="radio"
            :aria-checked="m.id === l.mode"
            :tabindex="m.id === l.mode ? 0 : -1"
            :class="[
              'flex h-14 items-center rounded-key px-3 text-left font-semibold ring-1 transition-[background-color,box-shadow] duration-150 ease-actuation active:scale-[0.98]',
              m.id === l.mode ? 'bg-wheel text-skate shadow-lift ring-wheel' : 'bg-skate ring-shell-deep hover:ring-wheel-muted/60',
              focusRing,
            ]"
            @click="dev.update({ lighting: { mode: m.id } })"
          >
            {{ modeName(m.label) }}
          </button>
        </div>
        <div class="grid grid-cols-2 gap-4 rounded-panel bg-skate p-6 shadow-lift ring-1 ring-shell-deep @max-4xl:grid-cols-1">
          <div :class="!f.color && 'opacity-45'">
            <h2 class="font-semibold">{{ t("lighting.color") }}</h2>
            <div role="radiogroup" :aria-label="t('lighting.lightColor')" class="mt-3 flex flex-wrap gap-2">
              <button
                v-for="c in COLORS"
                :key="c"
                type="button"
                role="radio"
                :aria-checked="c === l.color.toUpperCase()"
                :aria-label="t('common.color', { hex: c })"
                :disabled="!f.color"
                :style="{ '--stage': c }"
                :class="['lens size-7 rounded-full disabled:cursor-not-allowed', c === l.color.toUpperCase() && 'ring-2 ring-wheel ring-offset-2 ring-offset-skate', focusRing]"
                @click="dev.update({ lighting: { color: c } })"
              />
            </div>
            <p v-if="!f.color" class="mt-2 text-micro text-wheel-muted">{{ f.off ? t("lighting.off") : t("lighting.ownColors", { mode: modeLabel }) }}</p>
          </div>
          <div class="flex flex-col gap-4">
            <div :class="!f.brightness && 'opacity-45'">
              <div class="flex justify-between">
                <h2 class="font-semibold">{{ t("lighting.brightness") }}</h2>
                <span class="num">{{ t("lighting.of", { value: l.brightness, max: caps.brightness.max }) }}</span>
              </div>
              <RailInput :min="caps.brightness.min" :max="caps.brightness.max" :value="l.brightness" :label="t('lighting.brightness')" :disabled="!f.brightness" @input="(v) => dev.update({ lighting: { brightness: v } }, true)" @commit="() => dev.flush()" />
              <p v-if="!f.brightness" class="text-micro text-wheel-muted">{{ f.off ? t("lighting.off") : t("lighting.fixedBrightness", { mode: modeLabel }) }}</p>
            </div>
            <div :class="!f.speed && 'opacity-45'">
              <div class="flex justify-between">
                <h2 class="font-semibold">{{ t("lighting.speed") }}</h2>
                <span>{{ speedLabel }}</span>
              </div>
              <RailInput :min="caps.lightSpeed.min" :max="caps.lightSpeed.max" :value="l.speed" :label="t('lighting.speed')" :disabled="!f.speed" @input="(v) => dev.update({ lighting: { speed: v } }, true)" @commit="() => dev.flush()" />
              <p v-if="!f.speed" class="text-micro text-wheel-muted">{{ f.off ? t("lighting.off") : t("lighting.noSpeed", { mode: modeLabel }) }}</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
