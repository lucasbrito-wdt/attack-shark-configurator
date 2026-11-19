<script setup lang="ts">
// Stroke glyphs drawn for this app (24px grid, 1.6 stroke), shared by every screen.
const ICON = {
  overview: '<rect x="7" y="2.5" width="10" height="19" rx="5"/><path d="M12 2.5v5.5"/><path d="M12 11v1.5"/>',
  dpi: '<circle cx="6" cy="6" r="2"/><circle cx="6" cy="12" r="2"/><circle cx="6" cy="18" r="2"/><path d="M11 6h9M11 12h6M11 18h3"/>',
  tracking: '<circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="2.5"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3"/>',
  buttons: '<rect x="6" y="2.5" width="12" height="19" rx="6"/><path d="M12 2.5V10M6 10h12"/><path d="M6.6 6.5 11 3" />',
  macros: '<rect x="3" y="5" width="18" height="14" rx="3.5"/><path d="M7 15l2.5-5 2.5 5 2.5-5 2.5 5"/>',
  lighting: '<circle cx="12" cy="12" r="3.2"/><path d="M12 3v2.5M12 18.5V21M3 12h2.5M18.5 12H21M5.6 5.6l1.8 1.8M16.6 16.6l1.8 1.8M5.6 18.4l1.8-1.8M16.6 7.4l1.8-1.8"/>',
  power: '<rect x="2.5" y="7" width="17" height="10" rx="2.6"/><path d="M21.5 10.5v3M6 10.5v3M9 10.5v3M12 10.5v3"/>',
  chevron: '<path d="m7 10 5 5 5-5"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  minus: '<path d="M5 12h14"/>',
  ble: '<path d="m7 7 10 10-5 5V2l5 5L7 17"/>',
  dongle: '<rect x="8" y="2.5" width="8" height="10" rx="1.6"/><rect x="6.5" y="12" width="11" height="9.5" rx="2.4"/><path d="M10.5 6h3"/>',
  wired: '<path d="M12 21v-5M8 3v5M16 3v5M6 8h12v3a6 6 0 0 1-12 0z"/>',
  none: '<path d="M4 4l16 16M9 5.5A8 8 0 0 1 20 12M5 9.5A8 8 0 0 0 12 20"/>',
  sleep: '<path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z"/>',
  alert: '<path d="M12 8v5M12 16.5v.5"/><path d="M10.3 3.9 2.6 17.5A2 2 0 0 0 4.3 20.5h15.4a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z"/>',
  up: '<path d="m6 15 6-6 6 6"/>',
  down: '<path d="m6 9 6 6 6-6"/>',
  trash: '<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>',
  record: '<circle cx="12" cy="12" r="6"/>',
  stop: '<rect x="7" y="7" width="10" height="10" rx="1.5"/>',
  key: '<rect x="3" y="6" width="18" height="12" rx="3"/><path d="M7 10h.5M11 10h.5M15 10h.5M8 14h8"/>',
  clock: '<circle cx="12" cy="12" r="8"/><path d="M12 8v4.5l3 2"/>',
  pressed: '<path d="M12 5v12M6.5 11.5 12 17l5.5-5.5"/>',
  released: '<path d="M12 19V7M6.5 12.5 12 7l5.5 5.5"/>',
  read: '<path d="M12 3v11M7.5 9.5 12 14l4.5-4.5"/><path d="M4 16v2.5A2.5 2.5 0 0 0 6.5 21h11a2.5 2.5 0 0 0 2.5-2.5V16"/>',
  globe: '<circle cx="12" cy="12" r="9"/><path d="M3 12h18"/><path d="M12 3c2.5 2.6 3.8 5.6 3.8 9s-1.3 6.4-3.8 9c-2.5-2.6-3.8-5.6-3.8-9S9.5 5.6 12 3z"/>',
} as const;

export type IconName = keyof typeof ICON;

defineProps<{ name: IconName }>();
</script>

<template>
  <svg
    viewBox="0 0 24 24"
    class="shrink-0"
    fill="none"
    stroke="currentColor"
    stroke-width="1.6"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    v-html="ICON[name]"
  />
</template>
