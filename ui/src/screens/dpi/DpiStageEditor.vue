<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import AppIcon from "../../components/ui/AppIcon.vue";
import RailInput from "../../components/ui/RailInput.vue";
import { dpiToPos, posToDpi, snapDpi, stepAt } from "../../lib/caps";
import { btnGhost, btnIcon, btnPrimary, fmt, focusRing } from "../../lib/styles";
import { useDevice } from "../../stores/device";

const props = defineProps<{ index: number; on: boolean }>();
const emit = defineEmits<{ toggle: [] }>();

/** Factory stage colors; any other color comes from the custom picker. */
const PRESETS = ["#FF0000", "#00FF00", "#0000FF", "#FFFF00", "#00FFFF", "#FF00FF", "#FF4000", "#FFFFFF"];

const dev = useDevice();
const { t } = useI18n();
const caps = computed(() => dev.caps!);
const p = computed(() => dev.profile!);
const s = computed(() => p.value.dpiStages[props.index]);
const active = computed(() => props.index === p.value.activeStage && props.on);
const text = ref("");
const error = ref<string | null>(null);
/** While typing, the field keeps what was typed; the ladder shows the snapped value live. */
const typing = ref(false);
watch(() => s.value.dpi, (v) => !typing.value && (text.value = String(v)), { immediate: true });

const ticks = computed(() => {
  const c = caps.value;
  const marks = [1600, 3200].filter((v) => v > c.dpiMin && v < c.dpiMax);
  return marks.map((v) => ({ v, left: `${dpiToPos(c, v) / 10}%` }));
});
const hint = computed(() => {
  const c = caps.value;
  const range = { min: fmt(c.dpiMin), max: fmt(c.dpiMax), step: fmt(c.dpiStep) };
  return c.dpiHighStepFrom != null ? t("dpi.hintHigh", { ...range, high: fmt(100), from: fmt(c.dpiHighStepFrom) }) : t("dpi.hint", range);
});

function setDpi(v: number, defer = false) {
  error.value = null;
  dev.update({ dpiStages: [{ index: props.index, dpi: snapDpi(caps.value, v) }] }, defer);
}

function onText(v: string) {
  text.value = v;
  const n = Number(v.replace(/[^\d]/g, ""));
  const c = caps.value;
  if (!n || n < c.dpiMin || n > c.dpiMax) {
    error.value = t("dpi.range", { min: fmt(c.dpiMin), max: fmt(c.dpiMax) });
    return;
  }
  setDpi(n, true);
}

function onTextKey(e: KeyboardEvent) {
  if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
  e.preventDefault();
  const step = stepAt(caps.value, s.value.dpi) * (e.shiftKey ? 10 : 1);
  setDpi(s.value.dpi + (e.key === "ArrowUp" ? step : -step), true);
  text.value = String(s.value.dpi);
}

function nudge(sign: 1 | -1) {
  setDpi(s.value.dpi + sign * stepAt(caps.value, s.value.dpi + (sign < 0 ? -1 : 0)));
}

function makeActive() {
  const patch: Parameters<typeof dev.update>[0] = { activeStage: props.index };
  if (!props.on) patch.dpiStages = [{ index: props.index, enabled: true, dpi: s.value.dpi || snapDpi(caps.value, 1600) }];
  dev.update(patch);
}
</script>

<template>
  <section :aria-label="t('dpi.stageSettings', { n: index + 1 })" class="self-start rounded-panel bg-skate p-6 shadow-lift ring-1 ring-shell-deep" :style="{ '--stage': s.color }">
    <div class="flex items-center gap-3">
      <span class="lens size-5 rounded-full" />
      <h2 class="font-semibold">{{ t("dpi.stage", { n: index + 1 }) }}</h2>
      <span class="ml-auto text-micro text-wheel-muted">{{ !on ? t("common.off") : active ? t("dpi.activeOnMouse") : t("common.on") }}</span>
    </div>
    <label class="mt-5 flex items-baseline gap-2">
      <span class="sr-only">{{ t("dpi.value") }}</span>
      <input
        type="text"
        inputmode="numeric"
        :value="text"
        aria-describedby="dpi-hint dpi-error"
        :aria-invalid="!!error"
        :class="['num w-[5ch] min-w-0 flex-1 rounded-key bg-transparent text-numeral font-[260] tracking-[-0.03em] outline-none focus-visible:outline-offset-4', focusRing]"
        @input="onText(($event.target as HTMLInputElement).value)"
        @keydown="onTextKey"
        @focus="typing = true"
        @blur="typing = false; text = String(s.dpi); error = null"
      />
      <span class="wide text-lead text-wheel-muted">DPI</span>
    </label>
    <div class="mt-4 flex items-center gap-2">
      <button type="button" :aria-label="t('dpi.lower')" :class="[btnIcon, 'ring-1 ring-shell-deep']" :disabled="s.dpi <= caps.dpiMin" @click="nudge(-1)"><AppIcon name="minus" class="size-4" /></button>
      <RailInput
        :min="0"
        :max="1000"
        :value="dpiToPos(caps, s.dpi || caps.dpiMin)"
        :label="t('dpi.stageDpi', { n: index + 1 })"
        @input="(v) => setDpi(posToDpi(caps, v), true)"
        @commit="() => dev.flush()"
      />
      <button type="button" :aria-label="t('dpi.raise')" :class="[btnIcon, 'ring-1 ring-shell-deep']" :disabled="s.dpi >= caps.dpiMax" @click="nudge(1)"><AppIcon name="plus" class="size-4" /></button>
    </div>
    <div class="num relative mx-11 mt-1 h-4 text-micro text-wheel-muted" aria-hidden="true">
      <span class="absolute left-0">{{ fmt(caps.dpiMin) }}</span>
      <span v-for="tk in ticks" :key="tk.v" class="absolute -translate-x-1/2" :style="{ left: tk.left }">{{ fmt(tk.v) }}</span>
      <span class="absolute right-0">{{ fmt(caps.dpiMax) }}</span>
    </div>
    <p id="dpi-hint" class="mt-3 text-micro text-wheel-muted">{{ hint }}</p>
    <p id="dpi-error" :class="['mt-1 text-micro font-semibold text-sensor', !error && 'hidden']" role="alert">{{ error }}</p>
    <h3 class="mt-5 text-micro text-wheel-muted">{{ t("dpi.ledColor") }}</h3>
    <div role="radiogroup" :aria-label="t('dpi.stageColor')" class="mt-2 flex flex-wrap items-center gap-2">
      <button
        v-for="c in PRESETS"
        :key="c"
        type="button"
        role="radio"
        :aria-checked="c === s.color.toUpperCase()"
        :aria-label="t('common.color', { hex: c })"
        :style="{ '--stage': c }"
        :class="['lens size-7 rounded-full transition-transform duration-150 ease-actuation hover:scale-110 active:scale-95', c === s.color.toUpperCase() && 'ring-2 ring-wheel ring-offset-2 ring-offset-skate', focusRing]"
        @click="dev.update({ dpiStages: [{ index, color: c }] })"
      />
      <label class="relative grid size-7 cursor-pointer place-items-center rounded-full ring-1 ring-shell-deep hover:ring-wheel-muted focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-led-deep" :title="t('common.customColor')">
        <AppIcon name="plus" class="size-4 text-wheel-muted" />
        <input type="color" :value="s.color.toLowerCase()" class="absolute inset-0 cursor-pointer opacity-0" :aria-label="t('common.customColor')" @input="dev.update({ dpiStages: [{ index, color: ($event.target as HTMLInputElement).value.toUpperCase() }] }, true)" />
      </label>
    </div>
    <div class="mt-6 flex gap-2">
      <button type="button" :class="[btnPrimary, 'flex-1 whitespace-nowrap']" :disabled="active" @click="makeActive">{{ active ? t("dpi.activeStage") : t("dpi.makeActive") }}</button>
      <button type="button" :class="btnGhost" @click="emit('toggle')">{{ on ? t("dpi.turnOff") : t("dpi.turnOn") }}</button>
    </div>
  </section>
</template>
