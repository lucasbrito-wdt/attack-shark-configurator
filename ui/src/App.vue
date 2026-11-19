<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch, type Component } from "vue";
import { useI18n } from "vue-i18n";
import AppSidebar from "./components/shell/AppSidebar.vue";
import ApplyDock from "./components/shell/ApplyDock.vue";
import ConfirmDialog from "./components/shell/ConfirmDialog.vue";
import ToastStack from "./components/shell/ToastStack.vue";
import UpdateDialog from "./components/shell/UpdateDialog.vue";
import StageLight from "./components/stage/StageLight.vue";
import { useUpdater } from "./composables/useUpdater";
import ButtonsScreen from "./screens/ButtonsScreen.vue";
import DpiScreen from "./screens/DpiScreen.vue";
import LightingScreen from "./screens/LightingScreen.vue";
import MacrosScreen from "./screens/MacrosScreen.vue";
import OverviewScreen from "./screens/OverviewScreen.vue";
import PowerScreen from "./screens/PowerScreen.vue";
import TrackingScreen from "./screens/TrackingScreen.vue";
import { useDevice } from "./stores/device";
import { SCREENS, useUi, type ScreenId } from "./stores/ui";

const dev = useDevice();
const ui = useUi();
const { t } = useI18n();
const scroller = ref<HTMLElement | null>(null);

const VIEWS: Record<ScreenId, Component> = {
  overview: OverviewScreen, dpi: DpiScreen, tracking: TrackingScreen, buttons: ButtonsScreen,
  macros: MacrosScreen, lighting: LightingScreen, power: PowerScreen,
};
const view = computed(() => VIEWS[ui.screen]);
watch(() => ui.screen, () => scroller.value?.scrollTo({ top: 0 }));

// Cmd/Ctrl + 1..7 switches screen, Cmd/Ctrl + Return applies. Suspended while a capture owns the keyboard.
function onKey(e: KeyboardEvent) {
  if (ui.keyboardBusy || ui.confirmReq) return;
  const mod = e.metaKey || e.ctrlKey;
  if (mod && /^[1-7]$/.test(e.key)) {
    e.preventDefault();
    ui.go(SCREENS[Number(e.key) - 1].id);
  } else if (mod && e.key === "Enter" && dev.hasPending) {
    e.preventDefault();
    void dev.applyAll();
  }
}
onMounted(() => {
  window.addEventListener("keydown", onKey);
  void dev.init();
  useUpdater().start();
});
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <div class="relative flex h-dvh min-h-[640px] min-w-[960px] overflow-hidden text-body text-wheel">
    <AppSidebar />
    <main class="@container relative z-10 flex min-w-0 flex-1 flex-col bg-shell">
      <div data-tauri-drag-region class="absolute inset-x-0 top-0 z-[4] h-6" />
      <StageLight v-if="dev.ready" />
      <ToastStack />
      <div id="screen" ref="scroller" class="relative z-[3] min-h-0 flex-1 overflow-y-auto px-8 pb-6 pt-8">
        <component :is="view" v-if="dev.ready" />
        <div v-else-if="dev.loadError" class="grid h-full place-items-center">
          <div class="max-w-[48ch] text-center">
            <h1 class="wide text-heading font-[350]">{{ t("app.startFailed") }}</h1>
            <p class="mt-2 text-wheel-muted">{{ dev.loadError }}</p>
          </div>
        </div>
        <div v-else class="grid h-full place-items-center text-wheel-muted" role="status">
          <span class="flex items-center gap-3"><span class="spin size-5 rounded-full border-2 border-shell-deep border-t-wheel" />{{ t("app.looking") }}</span>
        </div>
      </div>
      <!-- in flow, not floating: the scroller ends above the dock so nothing can sit under it -->
      <div v-if="dev.ready" class="relative z-30 shrink-0 px-6 pb-5"><ApplyDock /></div>
    </main>
    <ConfirmDialog />
    <UpdateDialog />
  </div>
</template>
