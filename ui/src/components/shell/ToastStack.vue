<script setup lang="ts">
import { animate } from "motion-v";
import { useUi } from "../../stores/ui";
import AppIcon from "../ui/AppIcon.vue";

const ui = useUi();
const reduce = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;
const EASE = [0.2, 0.8, 0.12, 1] as const;

function enter(el: Element, done: () => void) {
  animate(el, { opacity: [0, 1], y: [-8, 0] }, { duration: reduce ? 0 : 0.35, ease: EASE }).then(done);
}
function leave(el: Element, done: () => void) {
  animate(el, { opacity: 0 }, { duration: reduce ? 0 : 0.3 }).then(done);
}
</script>

<template>
  <div class="pointer-events-none absolute inset-x-0 top-4 z-40 flex flex-col items-center gap-2" role="status" aria-live="polite">
    <TransitionGroup :css="false" @enter="enter" @leave="leave">
      <div
        v-for="t in ui.toasts"
        :key="t.id"
        :class="['pointer-events-auto flex items-center gap-3 rounded-full bg-skate/95 py-2 pl-2 pr-5 shadow-float ring-1 ring-shell-deep backdrop-blur-xl', t.kind === 'error' && 'text-sensor']"
      >
        <span v-if="t.color" class="lens lens-glow size-5 rounded-full" :style="{ '--stage': t.color }" />
        <span v-else :class="['grid size-6 place-items-center rounded-full text-skate', t.kind === 'error' ? 'bg-sensor' : 'bg-wheel']">
          <AppIcon :name="t.kind === 'error' ? 'alert' : 'check'" class="size-3" />
        </span>
        <span class="max-w-[60ch] font-semibold">{{ t.text }}</span>
      </div>
    </TransitionGroup>
  </div>
</template>
