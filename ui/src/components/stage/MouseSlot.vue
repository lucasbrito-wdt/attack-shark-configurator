<script setup lang="ts">
// Where the travelling mouse lands on a screen. The box takes the model art's aspect ratio, so anything
// placed inside it by percent (button hotspots) sits on the drawn mouse.
import { computed } from "vue";
import { artFor } from "../../lib/models";
import { useDevice } from "../../stores/device";

const dev = useDevice();
const art = computed(() => artFor(dev.model?.slug ?? ""));
</script>

<template>
  <div data-mouse-slot class="relative" :style="{ aspectRatio: `${art.w} / ${art.h}` }">
    <slot :art="art" />
  </div>
</template>
