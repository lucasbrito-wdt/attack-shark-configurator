<script setup lang="ts">
// Side view of the mouse hovering at the chosen lift-off distance; one dashed cut-off line per option.
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { fmt } from "../../lib/styles";

const props = defineProps<{ lod: number; options: number[] }>();
const { t } = useI18n();
const GROUND = 112;
/** px per mm, so the tallest option still fits under the 132-high box */
const scale = computed(() => Math.min(14, 56 / Math.max(...props.options, 1)));
const lift = computed(() => props.lod * scale.value);
const ticks = Array.from({ length: 26 }, (_, i) => 10 + i * 8);
</script>

<template>
  <svg viewBox="0 0 220 132" class="w-full" role="img" :aria-label="t('tracking.lifted', { n: fmt(lod) }, lod)">
    <defs>
      <linearGradient id="lift-shell" x1="0" y1="0" x2="0" y2="1"><stop offset="0" style="stop-color: var(--color-skate)" /><stop offset="1" style="stop-color: var(--color-shell-deep)" /></linearGradient>
    </defs>
    <path :d="`M8 ${GROUND} H212`" class="stroke-wheel-muted" stroke-width="1.4" />
    <path v-for="x in ticks" :key="x" :d="`M${x} ${GROUND} l-4 6`" class="stroke-shell-deep" stroke-width="1.2" />
    <path
      v-for="o in options"
      :key="o"
      :d="`M8 ${GROUND - o * scale} H212`"
      class="stroke-led-deep"
      stroke-dasharray="4 4"
      stroke-width="1.4"
      :opacity="o === lod ? 1 : 0.25"
    />
    <text x="8" :y="GROUND - lift - 6" class="fill-led-deep text-[10px] font-semibold">{{ fmt(lod) }} mm</text>
    <g class="transition-transform duration-[420ms] ease-actuation motion-reduce:transition-none" :style="{ transform: `translateY(${-lift + 10}px)` }">
      <path d="M36 100 C36 92 44 84 62 78 C92 68 134 56 164 64 C184 70 194 84 194 96 C194 100 190 102 184 102 H42 C38 102 36 102 36 100 Z" fill="url(#lift-shell)" class="stroke-wheel-muted" stroke-width="1.2" />
      <path d="M62 78 C70 82 80 86 96 86" class="stroke-wheel-muted" fill="none" stroke-width="1" />
      <rect x="104" y="100" width="20" height="3" rx="1.5" class="fill-wheel" />
      <path :d="`M114 103 L108 ${GROUND - 20 + lift} M114 103 L120 ${GROUND - 20 + lift}`" class="stroke-led" stroke-width="1.2" opacity="0.8" />
    </g>
  </svg>
</template>
