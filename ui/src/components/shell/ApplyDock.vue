<script setup lang="ts">
// The only primary action of every screen: says what is on the mouse, what is still only in the profile,
// shows the write with its retries, and explains why it cannot write (asleep, no mouse, unverified DPI).
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { MOD_KEY } from "../../lib/hid";
import { btnGhost, btnPrimary } from "../../lib/styles";
import { useDevice } from "../../stores/device";
import { useUi } from "../../stores/ui";
import AppIcon from "../ui/AppIcon.vue";

const dev = useDevice();
const ui = useUi();
const { t } = useI18n();
const d = dev.dock;
const name = computed(() => dev.profile?.name ?? "");
const groups = computed(() => dev.dirtyGroups.map((g) => t(`dock.group.${g}`)));
const count = computed(() => groups.value.length);
const pendingLine = computed(() => {
  if (!dev.linked) return t("dock.savedConnect");
  if (!dev.awake) return t("dock.asleep");
  return t("dock.shortcut", { keys: `${MOD_KEY}+Return` });
});
const view = computed(() => {
  if (d.state === "writing" || d.state === "done" || d.state === "failed" || d.state === "blocked") return d.state;
  return dev.hasPending ? "pending" : "synced";
});
</script>

<template>
  <div class="flex min-h-16 items-center gap-4 rounded-panel bg-skate/88 px-4 py-3 shadow-float ring-1 ring-shell-deep backdrop-blur-xl" aria-live="polite">
    <template v-if="view === 'writing'">
      <div class="flex min-w-0 items-center gap-3">
        <span class="spin grid size-8 shrink-0 place-items-center rounded-full border-2 border-shell-deep border-t-wheel" />
        <span>
          <span class="block font-semibold">{{ t("dock.writing") }}</span>
          <span class="block text-micro text-wheel-muted">{{ d.attempt ? t("dock.retry", { attempt: d.attempt + 1, total: 3 }) : d.packets ? t("dock.waitingConfirm", { n: d.packets }, d.packets) : t("dock.waitingPrevious") }}</span>
        </span>
      </div>
      <div class="flex min-w-0 flex-1 flex-wrap justify-end gap-1 @max-4xl:hidden">
        <span v-for="g in groups" :key="g" class="flex h-7 items-center rounded-full bg-shell px-3 text-micro font-semibold text-wheel-muted">{{ g }}</span>
      </div>
      <div class="ml-auto flex shrink-0 items-center gap-2"><button type="button" :class="btnPrimary" disabled>{{ t("dock.applying") }}</button></div>
    </template>

    <template v-else-if="view === 'done'">
      <div class="flex min-w-0 items-center gap-3">
        <span class="grid size-8 shrink-0 place-items-center rounded-full bg-wheel text-skate"><AppIcon name="check" class="size-4" /></span>
        <span class="font-semibold">{{ d.message }}</span>
      </div>
    </template>

    <template v-else-if="view === 'failed' || view === 'blocked'">
      <div class="flex min-w-0 flex-1 items-center gap-3">
        <span :class="['grid size-8 shrink-0 place-items-center rounded-full text-skate', d.kind ? 'bg-led-deep' : 'bg-sensor']"><AppIcon :name="view === 'blocked' ? 'sleep' : 'alert'" class="size-4" /></span>
        <span class="min-w-0">
          <span :class="['block font-semibold', d.kind ? 'text-wheel' : 'text-sensor']">{{ d.message }}</span>
        </span>
      </div>
      <div class="ml-auto flex shrink-0 items-center gap-2">
        <button v-if="d.kind === 'unverified'" type="button" :class="btnGhost" @click="ui.go('dpi')">{{ t("dock.seeDetails") }}</button>
        <button v-if="d.kind !== 'unverified'" type="button" :class="btnPrimary" :disabled="!dev.awake" @click="dev.applyAll()">{{ t("dock.tryAgain") }}</button>
      </div>
    </template>

    <template v-else-if="view === 'pending'">
      <div class="flex min-w-0 items-center gap-3">
        <span class="relative grid size-8 shrink-0 place-items-center rounded-full bg-led/15 text-led-deep">
          <span v-if="count" class="num text-micro font-bold">{{ count }}</span><span v-else class="size-2 rounded-full bg-led-deep" />
        </span>
        <span>
          <span class="block font-semibold">{{ !count ? (dev.dpiHeld ? t("dock.dpiNotOnMouse") : t("dock.profileNotOnMouse", { name })) : t("dock.changesNotOnMouse", { n: count }, count) }}</span>
          <span class="block text-micro text-wheel-muted">{{ !count && dev.dpiHeld ? t("dock.dpiHeldHint", { model: dev.model?.name ?? "" }) : pendingLine }}</span>
        </span>
      </div>
      <div class="flex min-w-0 flex-1 flex-wrap justify-end gap-1 @max-4xl:hidden">
        <span v-for="g in groups" :key="g" class="flex h-7 items-center rounded-full bg-shell px-3 text-micro font-semibold">{{ g }}</span>
      </div>
      <div class="ml-auto flex shrink-0 items-center gap-2">
        <button type="button" :class="btnPrimary" :disabled="!dev.linked || !dev.awake" @click="dev.applyAll()">{{ t("dock.apply") }}</button>
      </div>
    </template>

    <template v-else>
      <div class="flex min-w-0 items-center gap-3">
        <span class="grid size-8 shrink-0 place-items-center rounded-full bg-shell text-wheel-muted"><AppIcon :name="dev.linked ? 'check' : 'none'" class="size-4" /></span>
        <span class="text-wheel-muted">{{ dev.linked ? t("dock.synced", { name }) : t("dock.readyNoMouse", { name }) }}</span>
      </div>
    </template>
  </div>
</template>
