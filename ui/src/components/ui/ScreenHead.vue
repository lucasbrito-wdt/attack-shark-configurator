<script setup lang="ts">
defineProps<{ title: string; sub?: string }>();
</script>

<template>
  <header class="mb-6 flex items-end justify-between gap-6">
    <div>
      <h1 class="wide text-heading font-[350] tracking-[-0.01em]">{{ title }}</h1>
      <p v-if="sub" class="mt-2 max-w-[60ch] text-wheel-muted">{{ sub }}</p>
    </div>
    <!-- actions keep one line however long the translated label -->
    <div class="flex shrink-0 gap-2 whitespace-nowrap"><slot /></div>
  </header>
</template>
