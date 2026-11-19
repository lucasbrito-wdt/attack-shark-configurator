<script setup lang="ts">
// Ten cells, one per 10 % step the mouse reports. Charging sweeps; unknown is dashed, never a guess.
defineProps<{ level: number | null; charging: boolean }>();
</script>

<template>
  <div class="flex gap-0.5" aria-hidden="true">
    <template v-for="i in 10" :key="i">
      <span v-if="charging" class="seg-charging flex-1 rounded-[2px] bg-led" :style="{ animationDelay: `${(i - 1) * 90}ms` }" />
      <span v-else-if="level == null" class="flex-1 rounded-[2px] border border-dashed border-wheel-muted/40" />
      <span v-else :class="['flex-1 rounded-[2px]', i - 1 < level / 10 ? (level <= 20 ? 'bg-sensor' : 'bg-wheel') : 'bg-shell-deep']" />
    </template>
  </div>
</template>
