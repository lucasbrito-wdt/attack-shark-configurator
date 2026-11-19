<script setup lang="ts">
// The signature: one mouse that travels between the [data-mouse-slot] of each screen, its LED lit in the
// active stage color, a pool of that light on the window floor and, on DPI, a beam from the LED to the
// active stage row. When the mouse changes stage by itself the LED blinks and the pool re-blooms.
import { animate } from "motion-v";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { artFor, type ModelArt } from "../../lib/models";
import { useDevice } from "../../stores/device";
import { useUi } from "../../stores/ui";
import MouseIllustration from "./MouseIllustration.vue";

const EASE = [0.2, 0.8, 0.12, 1] as const; // --ease-actuation
const BASE_H = 720;

const dev = useDevice();
const { t } = useI18n();
const ui = useUi();
const root = ref<HTMLElement | null>(null);
const mouseEl = ref<HTMLElement | null>(null);
const poolEl = ref<HTMLElement | null>(null);
const beamPath = ref<SVGPathElement | null>(null);
const beamDot = ref<SVGCircleElement | null>(null);
const beamOn = ref(false);
const reduce = typeof matchMedia !== "undefined" && matchMedia("(prefers-reduced-motion: reduce)").matches;

const art = computed<ModelArt>(() => artFor(dev.model?.slug ?? ""));
const baseW = computed(() => (BASE_H * art.value.w) / art.value.h);
const ledOn = computed(() => dev.awake);
const title = computed(() => t("stage.topView", { model: dev.model?.name ?? "" }));

const lightColor = computed<string | null>(() => {
  const p = dev.profile;
  if (!p) return null;
  if (ui.screen === "lighting") {
    const label = dev.caps?.lightingModes.find((m) => m.id === p.lighting.mode)?.label ?? "";
    return !dev.caps?.lightingModes.length || /off/i.test(label) ? null : p.lighting.color;
  }
  return dev.awake ? (dev.activeStage?.color ?? null) : null;
});

// LED point in the mouse element's own pixels (before scale)
const ledLocal = computed(() => [(art.value.led[0] / art.value.w) * baseW.value, (art.value.led[1] / art.value.h) * BASE_H] as const);
const imgMask = computed(() => (art.value.src ? { maskImage: `url(${art.value.src})`, WebkitMaskImage: `url(${art.value.src})`, maskSize: "100% 100%", WebkitMaskSize: "100% 100%" } : {}));

let placed = false;
let beamY: number | null = null;

function slot(): HTMLElement | null {
  const el = document.querySelector<HTMLElement>("#screen [data-mouse-slot]");
  return el && el.offsetParent !== null && el.getBoundingClientRect().width > 0 ? el : null;
}

function place(instant = false) {
  const host = root.value, m = mouseEl.value, pool = poolEl.value;
  if (!host || !m || !pool) return;
  document.documentElement.style.setProperty("--stage", lightColor.value ?? "rgb(211 218 225)");
  const s = slot();
  const fast = instant || reduce || !placed;
  const opts = { duration: fast ? 0 : 0.7, ease: EASE };
  if (!s) {
    animate(m, { opacity: 0 }, { duration: instant ? 0 : 0.25 });
    animate(pool, { opacity: 0 }, { duration: instant ? 0 : 0.25 });
    drawBeam(null, 0);
    return;
  }
  const h = host.getBoundingClientRect(), r = s.getBoundingClientRect();
  const scale = r.height / BASE_H;
  const x = r.left - h.left + (r.width - baseW.value * scale) / 2;
  const y = r.top - h.top;
  animate(m, { x, y, scale, opacity: dev.linked ? 1 : 0.55 }, opts);
  const led = { x: x + ledLocal.value[0] * scale, y: y + ledLocal.value[1] * scale };
  const overview = ui.screen === "overview";
  animate(pool, { x: led.x - 480, y: led.y - 480, scale: overview ? 1 : 0.62, opacity: lightColor.value ? (overview ? 0.7 : 0.45) : 0 }, opts);
  placed = true;
  requestAnimationFrame(() => drawBeam(led, opts.duration));
}

function drawBeam(led: { x: number; y: number } | null, duration: number) {
  const p = dev.profile;
  const row = ui.screen === "dpi" && led && p ? document.querySelector<HTMLElement>(`#ladder [data-row="${p.activeStage}"]`) : null;
  if (!row || !root.value || !lightColor.value || !p?.dpiStages[p.activeStage]?.enabled) {
    beamOn.value = false;
    beamY = null;
    return;
  }
  beamOn.value = true;
  const h = root.value.getBoundingClientRect(), rr = row.getBoundingClientRect();
  const tx = rr.left - h.left, ty = rr.top - h.top + rr.height / 2;
  const set = (yy: number) => {
    const mx = led!.x + (tx - led!.x) * 0.55;
    beamPath.value?.setAttribute("d", `M${led!.x} ${led!.y} C${mx} ${led!.y} ${mx} ${yy} ${tx} ${yy}`);
    beamDot.value?.setAttribute("cx", String(tx));
    beamDot.value?.setAttribute("cy", String(yy));
  };
  const from = beamY ?? ty;
  beamY = ty;
  if (duration && from !== ty) animate(from, ty, { duration, ease: EASE, onUpdate: set });
  else set(ty);
}

const layoutKey = computed(() => {
  const p = dev.profile;
  return [ui.screen, dev.model?.slug, p?.id, p?.activeStage, p?.dpiStages.map((s) => +s.enabled).join(""), dev.linked, dev.awake, lightColor.value].join("|");
});

const schedule = (instant = false) => nextTick(() => requestAnimationFrame(() => place(instant)));
watch(layoutKey, () => schedule());

watch(
  () => ui.stagePulse,
  () => {
    if (reduce) return;
    requestAnimationFrame(() => {
      const led = mouseEl.value?.querySelector("[data-led]");
      if (led) animate(led, { opacity: [1, 0.15, 1, 0.15, 1] }, { duration: 0.6 });
      if (poolEl.value) animate(poolEl.value, { scale: [0.5, ui.screen === "overview" ? 1 : 0.62] }, { duration: 0.9, ease: EASE });
    });
  },
);

let ro: ResizeObserver | null = null;
const onScroll = () => place(true);
let scroller: HTMLElement | null = null;
onMounted(() => {
  schedule(true);
  ro = new ResizeObserver(() => place(true));
  if (root.value) ro.observe(root.value);
  scroller = document.getElementById("screen");
  scroller?.addEventListener("scroll", onScroll, { passive: true });
});
onBeforeUnmount(() => {
  ro?.disconnect();
  scroller?.removeEventListener("scroll", onScroll);
});

defineExpose({ place: schedule });
</script>

<template>
  <div ref="root" class="pointer-events-none absolute inset-0" aria-hidden="true">
    <div ref="poolEl" class="pool absolute left-0 top-0 size-[960px] rounded-full opacity-0" />
    <svg class="absolute inset-0 z-[1] size-full overflow-visible">
      <defs>
        <linearGradient id="beam-g" x1="0" x2="1">
          <stop offset="0" stop-color="var(--stage)" stop-opacity="0.9" /><stop offset="1" stop-color="var(--stage)" stop-opacity="0.35" />
        </linearGradient>
      </defs>
      <g v-show="beamOn">
        <path ref="beamPath" fill="none" stroke="url(#beam-g)" stroke-width="3" stroke-linecap="round" style="filter: drop-shadow(0 0 6px var(--stage))" />
        <circle ref="beamDot" r="4" fill="var(--stage)" />
      </g>
    </svg>
    <div ref="mouseEl" class="absolute left-0 top-0 z-[2] origin-top-left opacity-0" :style="{ width: `${baseW}px`, height: `${BASE_H}px` }">
      <MouseIllustration v-if="!art.src" :led-on="ledOn" :title="title" />
      <template v-else>
        <img :src="art.src" :alt="title" class="size-full drop-shadow-[0_18px_28px_rgb(27_35_43/0.28)]" draggable="false" />
        <!-- LED light diffusing through the shell, clipped to the render's own silhouette -->
        <div v-if="ledOn" class="absolute inset-0" :style="imgMask">
          <div class="shell-glow absolute size-[240px] -translate-x-1/2 -translate-y-1/2 opacity-30" :style="{ left: `${ledLocal[0]}px`, top: `${ledLocal[1]}px` }" />
        </div>
        <div v-if="ledOn" class="absolute size-[44px] -translate-x-1/2 -translate-y-1/2 rounded-full opacity-55 blur-[12px]" :style="{ left: `${ledLocal[0]}px`, top: `${ledLocal[1]}px`, background: 'var(--stage)' }" />
        <span
          data-led
          class="absolute h-[12px] w-[26px] -translate-x-1/2 -translate-y-1/2 rounded-full"
          :style="{ left: `${ledLocal[0]}px`, top: `${ledLocal[1]}px`, background: ledOn ? 'var(--stage)' : '#2A3239' }"
        />
      </template>
    </div>
  </div>
</template>
