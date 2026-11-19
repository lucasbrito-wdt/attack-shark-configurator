<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useUpdater } from "../../composables/useUpdater";
import { IS_MAC, MOD_KEY } from "../../lib/hid";
import { focusRing } from "../../lib/styles";
import { useDevice } from "../../stores/device";
import { SCREENS, useUi } from "../../stores/ui";
import AppIcon, { type IconName } from "../ui/AppIcon.vue";
import BatteryIndicator from "./BatteryIndicator.vue";
import LanguageSelect from "./LanguageSelect.vue";
import ProfileSwitcher from "./ProfileSwitcher.vue";

const dev = useDevice();
const ui = useUi();
const { t } = useI18n();
const updater = useUpdater().state;

const mode = computed(() => (dev.status?.connected ? dev.status.mode : "none"));
// Over Bluetooth the system knows the mouse by its BLE name (e.g. "X6 Mouse2"); show that one there.
const bleName = computed(() => (mode.value === "ble" && dev.status?.model) || null);
const chipLine = computed(() => {
  if (!dev.linked) return t("sidebar.notConnected");
  if (!dev.awake) return t("sidebar.asleep");
  return t(`link.short.${mode.value}`);
});
</script>

<template>
  <aside class="relative z-20 flex w-[232px] shrink-0 flex-col border-r border-shell-deep/70 bg-skate/72 backdrop-blur-2xl max-[1099px]:w-[208px]">
    <!-- macOS draws the traffic lights over this strip; it also drags the window -->
    <div data-tauri-drag-region :class="IS_MAC ? 'h-13' : 'h-4'" />
    <ProfileSwitcher />
    <nav :aria-label="t('nav.sections')" class="mt-4 flex flex-col gap-0.5 px-3">
      <a
        v-for="(s, i) in SCREENS"
        :key="s.id"
        :href="`#${s.id}`"
        :aria-current="ui.screen === s.id ? 'page' : undefined"
        :title="t('nav.shortcut', { name: t(`nav.${s.id}`), keys: `${MOD_KEY}+${i + 1}` })"
        :class="[
          'group relative flex h-10 items-center gap-3 rounded-key px-3 font-medium transition-colors duration-150 ease-actuation',
          ui.screen === s.id ? 'bg-skate text-wheel shadow-lift ring-1 ring-shell-deep' : 'text-wheel-muted hover:bg-shell/80 hover:text-wheel',
          focusRing,
        ]"
        @click.prevent="ui.go(s.id)"
      >
        <AppIcon :name="s.id as IconName" class="size-[18px]" />
        <span>{{ t(`nav.${s.id}`) }}</span>
        <span v-if="s.id === 'dpi' && dev.activeStage" class="lens ml-auto size-2 rounded-full" :style="{ '--stage': dev.activeStage.color }" />
      </a>
    </nav>
    <div class="mt-auto flex flex-col gap-1 p-3">
      <button
        v-if="updater.phase === 'available' || updater.phase === 'downloading' || updater.phase === 'installing'"
        type="button"
        :title="`${t('update.pill')} v${updater.version}`"
        :class="['mb-1 flex h-8 items-center gap-2 self-start whitespace-nowrap rounded-full bg-led/15 px-3 text-micro font-semibold text-led-deep transition-colors duration-150 ease-actuation hover:bg-led/25', focusRing]"
        @click="updater.open = true"
      >
        <span class="size-1.5 rounded-full bg-led-deep" aria-hidden="true" />{{ t("update.pill") }}<span class="num font-medium max-[1099px]:sr-only">v{{ updater.version }}</span>
      </button>
      <BatteryIndicator />
      <LanguageSelect />
      <button
        type="button"
        :class="['flex w-full items-center gap-3 rounded-key bg-shell/60 p-3 text-left ring-1 ring-shell-deep transition-colors hover:bg-shell', focusRing]"
        @click="ui.go('overview')"
      >
        <span :class="['relative grid size-9 place-items-center rounded-full bg-skate shadow-lift', dev.linked ? 'text-wheel' : 'text-wheel-muted']">
          <AppIcon :name="mode" class="size-4" />
          <span v-if="dev.awake && dev.activeStage" class="lens absolute -right-0.5 -top-0.5 size-3 rounded-full ring-2 ring-skate" :style="{ '--stage': dev.activeStage.color }" />
        </span>
        <span class="min-w-0 flex-1">
          <span class="block truncate text-micro font-semibold">
            <template v-if="bleName">{{ bleName }}</template>
            <template v-else><span v-if="(dev.model?.name.length ?? 0) <= 8" class="max-[1099px]:hidden">ATTACK SHARK </span>{{ dev.model?.name }}</template>
          </span>
          <span class="block truncate text-micro text-wheel-muted">{{ chipLine }}</span>
        </span>
      </button>
    </div>
  </aside>
</template>
