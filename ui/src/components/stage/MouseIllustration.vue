<script setup lang="ts">
// Top view of the X3-shaped shell (viewBox 400 x 720), drawn from the vendor render's proportions.
// The LED sits under the wheel at (200, 257); StageLight reads that point for the pool and the beam.
defineProps<{ ledOn: boolean; title: string }>();

const SHELL = "M200 30 C286 30 340 72 348 168 C354 236 344 300 346 362 C348 428 368 486 370 566 C372 654 298 706 200 706 C102 706 28 654 30 566 C32 486 52 428 54 362 C56 300 46 236 52 168 C60 72 114 30 200 30 Z";
const LEFT_BTN = "M200 32 L200 352 C150 350 100 342 51 318 C50 280 46 232 53 170 C60 74 114 32 200 32 Z";
const RIGHT_BTN = "M200 32 L200 352 C250 350 300 342 349 318 C350 280 354 232 347 170 C340 74 286 32 200 32 Z";
const SEAM = "M51 318 C100 342 150 350 200 352 C250 350 300 342 349 318";
const ribs = Array.from({ length: 13 }, (_, i) => ({ y: 98 + i * 8.6, o: 0.35 + Math.sin((i / 12) * Math.PI) * 0.55 }));
const c = { top: "#FFFFFF", mid: "#EEF1F4", edge: "#C9D1D9", deep: "#9AA6B2", seam: "#8E9AA6", well: "#1B232B", wheelA: "#3A444E", wheelB: "#151B21", side: "#D5DCE3" };
</script>

<template>
  <svg viewBox="0 0 400 720" width="100%" height="100%" overflow="visible" style="overflow: visible" role="img" :aria-label="title" xmlns="http://www.w3.org/2000/svg">
    <defs>
      <radialGradient id="x3-dome" cx="0.38" cy="0.3" r="0.85">
        <stop offset="0" :stop-color="c.top" /><stop offset="0.45" :stop-color="c.mid" /><stop offset="0.85" :stop-color="c.edge" /><stop offset="1" :stop-color="c.deep" />
      </radialGradient>
      <radialGradient id="x3-rim" cx="0.5" cy="0.46" r="0.62">
        <stop offset="0.6" :stop-color="c.deep" stop-opacity="0" /><stop offset="1" :stop-color="c.deep" stop-opacity="0.7" />
      </radialGradient>
      <linearGradient id="x3-btnL" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" :stop-color="c.top" stop-opacity="0.9" /><stop offset="1" :stop-color="c.top" stop-opacity="0" />
      </linearGradient>
      <linearGradient id="x3-btnR" x1="1" y1="0" x2="0" y2="1">
        <stop offset="0" :stop-color="c.deep" stop-opacity="0.18" /><stop offset="1" :stop-color="c.deep" stop-opacity="0" />
      </linearGradient>
      <linearGradient id="x3-wheel" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" :stop-color="c.wheelB" /><stop offset="0.5" :stop-color="c.wheelA" /><stop offset="1" :stop-color="c.wheelB" />
      </linearGradient>
      <linearGradient id="x3-side" x1="0" y1="0" x2="1" y2="0">
        <stop offset="0" :stop-color="c.deep" /><stop offset="1" :stop-color="c.side" />
      </linearGradient>
      <filter id="x3-blur30" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="30" /></filter>
      <filter id="x3-blur12" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="12" /></filter>
      <clipPath id="x3-clip"><path :d="SHELL" /></clipPath>
    </defs>

    <!-- ground shadow: wide ambient, then tight contact -->
    <ellipse cx="206" cy="392" rx="186" ry="330" fill="#1B232B" opacity="0.22" filter="url(#x3-blur30)" />
    <path :d="SHELL" transform="translate(4 10)" fill="#1B232B" opacity="0.25" filter="url(#x3-blur12)" />

    <!-- side buttons, only their outer lip shows past the shell -->
    <rect x="38" y="214" width="18" height="94" rx="8" fill="url(#x3-side)" :stroke="c.seam" stroke-opacity="0.5" />
    <rect x="40" y="318" width="18" height="96" rx="8" fill="url(#x3-side)" :stroke="c.seam" stroke-opacity="0.5" />

    <path :d="SHELL" fill="url(#x3-dome)" />
    <g clip-path="url(#x3-clip)">
      <path :d="LEFT_BTN" fill="url(#x3-btnL)" />
      <path :d="RIGHT_BTN" fill="url(#x3-btnR)" />
      <!-- light from the LED diffusing through the shell plastic -->
      <circle v-if="ledOn" cx="200" cy="257" r="120" style="fill: var(--stage)" opacity="0.16" filter="url(#x3-blur30)" />
      <ellipse cx="128" cy="168" rx="70" ry="120" fill="#FFFFFF" opacity="0.75" filter="url(#x3-blur30)" />
      <ellipse cx="150" cy="520" rx="90" ry="110" fill="#FFFFFF" opacity="0.45" filter="url(#x3-blur30)" />
      <path :d="SHELL" fill="url(#x3-rim)" />
    </g>

    <!-- seams: dark groove plus a lit lip below it -->
    <g fill="none" stroke-linecap="round">
      <path :d="SEAM" stroke="#FFFFFF" stroke-width="2" opacity="0.95" transform="translate(0 1.6)" />
      <path :d="SEAM" :stroke="c.seam" stroke-width="2.2" />
      <path d="M200 30 L200 78 M200 298 L200 352" stroke="#FFFFFF" stroke-width="1.6" opacity="0.9" transform="translate(1.6 0)" />
      <path d="M200 30 L200 78 M200 298 L200 352" :stroke="c.seam" stroke-width="2.2" />
      <path :d="SHELL" stroke="#FFFFFF" stroke-width="1.4" opacity="0.7" />
    </g>

    <!-- wheel well, wheel, LED -->
    <rect x="174" y="76" width="52" height="224" rx="26" :fill="c.well" />
    <rect x="174" y="76" width="52" height="224" rx="26" fill="none" stroke="#FFFFFF" stroke-opacity="0.8" stroke-width="1.4" transform="translate(0 1.4)" />
    <rect x="184" y="88" width="32" height="130" rx="15" fill="url(#x3-wheel)" />
    <rect v-for="(r, i) in ribs" :key="i" x="187" :y="r.y" width="26" height="3.2" rx="1.6" fill="#5B6670" :opacity="r.o" />
    <rect x="184" y="88" width="32" height="130" rx="15" fill="none" stroke="#FFFFFF" stroke-opacity="0.08" />
    <circle v-if="ledOn" cx="200" cy="257" r="22" style="fill: var(--stage)" opacity="0.55" filter="url(#x3-blur12)" />
    <rect data-led x="195.5" y="244" width="9" height="26" rx="4.5" :style="{ fill: ledOn ? 'var(--stage)' : '#2A3239' }" />
    <rect v-if="ledOn" x="197.5" y="247" width="3" height="10" rx="1.5" fill="#FFFFFF" opacity="0.7" />
  </svg>
</template>
