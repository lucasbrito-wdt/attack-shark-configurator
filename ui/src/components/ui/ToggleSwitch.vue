<script setup lang="ts">
import { focusRing } from "../../lib/styles";

defineProps<{ on: boolean; label: string; disabled?: boolean }>();
defineEmits<{ toggle: [] }>();
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="on"
    :aria-label="label"
    :disabled="disabled"
    :class="[
      'relative h-7 w-12 shrink-0 rounded-full transition-colors duration-200 ease-actuation disabled:cursor-not-allowed disabled:opacity-45',
      on ? 'bg-wheel' : 'bg-shell-deep hover:bg-wheel-muted/40',
      focusRing,
    ]"
    @click="$emit('toggle')"
  >
    <span :class="['absolute left-1 top-1 size-5 rounded-full bg-skate shadow-lift transition-transform duration-200 ease-actuation', on && 'translate-x-5']" />
  </button>
</template>
