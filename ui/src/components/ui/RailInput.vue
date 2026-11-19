<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{ min: number; max: number; step?: number; value: number; label: string; disabled?: boolean; fill?: string }>();
const emit = defineEmits<{ input: [value: number]; commit: [value: number] }>();
const pct = computed(() => `${((props.value - props.min) / (props.max - props.min || 1)) * 100}%`);
</script>

<template>
  <input
    type="range"
    class="rail"
    :min="min"
    :max="max"
    :step="step ?? 1"
    :value="value"
    :aria-label="label"
    :disabled="disabled"
    :style="{ '--pct': pct, ...(fill ? { '--fill': fill } : {}) }"
    @input="emit('input', Number(($event.target as HTMLInputElement).value))"
    @change="emit('commit', Number(($event.target as HTMLInputElement).value))"
  />
</template>
