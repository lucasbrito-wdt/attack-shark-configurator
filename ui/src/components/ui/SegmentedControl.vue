<script setup lang="ts" generic="T extends string | number">
import { focusRing } from "../../lib/styles";

export interface SegmentOption<V> {
  value: V;
  label: string;
  sub?: string;
}

defineProps<{ name: string; options: SegmentOption<T>[]; value: T; disabled?: boolean }>();
const emit = defineEmits<{ select: [value: T] }>();

// Arrow keys move the choice, like a native radio group.
function onKey(e: KeyboardEvent, options: SegmentOption<T>[], value: T) {
  const i = options.findIndex((o) => o.value === value);
  const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
  if (!d) return;
  e.preventDefault();
  const next = options[(i + d + options.length) % options.length];
  emit("select", next.value);
  const group = (e.currentTarget as HTMLElement).closest('[role="radiogroup"]');
  requestAnimationFrame(() => (group?.querySelector('[aria-checked="true"]') as HTMLElement | null)?.focus());
}
</script>

<template>
  <div role="radiogroup" :aria-label="name" :class="['flex rounded-key bg-shell p-1 shadow-press', disabled && 'opacity-45']">
    <button
      v-for="o in options"
      :key="String(o.value)"
      type="button"
      role="radio"
      :aria-checked="o.value === value"
      :tabindex="o.value === value ? 0 : -1"
      :disabled="disabled"
      :class="[
        'flex flex-1 flex-col items-center justify-center whitespace-nowrap rounded-[7px] px-3 py-2 transition-[background-color,box-shadow,color] duration-200 ease-actuation disabled:cursor-not-allowed',
        o.value === value ? 'bg-skate text-wheel shadow-lift' : 'text-wheel-muted hover:text-wheel',
        focusRing,
      ]"
      @click="emit('select', o.value)"
      @keydown="onKey($event, options, value)"
    >
      <span :class="['font-semibold', o.sub && 'num text-lead']">{{ o.label }}</span>
      <span v-if="o.sub" class="text-micro">{{ o.sub }}</span>
    </button>
  </div>
</template>
