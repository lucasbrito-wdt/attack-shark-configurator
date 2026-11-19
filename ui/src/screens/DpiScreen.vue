<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import MouseSlot from "../components/stage/MouseSlot.vue";
import AppIcon from "../components/ui/AppIcon.vue";
import ScreenHead from "../components/ui/ScreenHead.vue";
import ToggleSwitch from "../components/ui/ToggleSwitch.vue";
import { snapDpi, unverifiedFrom } from "../lib/caps";
import { btnGhost, btnSecondary, fmt, focusRing } from "../lib/styles";
import { useDevice } from "../stores/device";
import { useUi } from "../stores/ui";
import DpiStageEditor from "./dpi/DpiStageEditor.vue";

const dev = useDevice();
const ui = useUi();
const { t } = useI18n();
const p = computed(() => dev.profile!);
const caps = computed(() => dev.caps!);
const stages = computed(() => p.value.dpiStages.slice(0, caps.value.maxDpiStages));
const isOn = (i: number) => !!stages.value[i]?.enabled && stages.value[i].dpi > 0;
const onCount = computed(() => stages.value.filter((_, i) => isOn(i)).length);
const selected = computed(() => {
  const s = ui.selection.stage;
  return s != null && s < stages.value.length ? s : p.value.activeStage;
});
/** Set only when a stage that is on goes over the unverified "t" range. */
const overLimit = computed(() => {
  const lim = unverifiedFrom(caps.value);
  return lim != null && stages.value.some((s, i) => isOn(i) && s.dpi > lim) ? lim : null;
});
const sub = computed(
  () => t("dpi.sub", { model: dev.model?.name ?? "", n: caps.value.maxDpiStages }),
);

function toggle(i: number) {
  if (isOn(i)) {
    if (onCount.value === 1) return ui.toast("error", t("dpi.keepOne"));
    const patch: Parameters<typeof dev.update>[0] = { dpiStages: [{ index: i, enabled: false }] };
    if (p.value.activeStage === i) patch.activeStage = stages.value.findIndex((_, j) => j !== i && isOn(j));
    dev.update(patch);
  } else {
    // an unused slot has dpi 0: give it a value next to the last stage that is on
    const last = [...stages.value].reverse().find((s) => s.enabled && s.dpi > 0);
    const dpi = stages.value[i].dpi || snapDpi(caps.value, (last?.dpi ?? 1600) + 800);
    dev.update({ dpiStages: [{ index: i, enabled: true, dpi }] });
  }
}

function addStage() {
  const i = stages.value.findIndex((_, j) => !isOn(j));
  if (i < 0) return;
  toggle(i);
  ui.selection.stage = i;
}
</script>

<template>
  <div>
    <ScreenHead :title="t('nav.dpi')" :sub="sub">
      <button type="button" :class="btnSecondary" :disabled="onCount >= stages.length" :title="onCount >= stages.length ? t('dpi.allOn', { n: stages.length }) : undefined" @click="addStage">
        <AppIcon name="plus" class="size-4" />{{ t("dpi.addStage") }}
      </button>
    </ScreenHead>

    <div v-if="!caps.dpiVerified || overLimit || dev.dpiHeld" class="-mt-2 mb-5 flex max-w-[84ch] items-start gap-3 rounded-key bg-skate/70 px-4 py-3 text-micro ring-1 ring-shell-deep" role="note">
      <AppIcon name="alert" class="size-4 shrink-0 text-led-deep" />
      <p class="min-w-0 flex-1 text-wheel-muted">
        <span v-if="dev.dpiHeld" class="block font-semibold text-wheel">{{ t("dpi.held") }}</span>
        <template v-if="!caps.dpiVerified">
          <span class="block">{{ t("dpi.locked", { model: caps.modelName }) }}</span>
          <span v-if="dev.verifyingDpi" class="mt-2 flex items-center gap-2 font-semibold text-wheel" role="status">
            <span class="spin size-3.5 rounded-full border-2 border-shell-deep border-t-wheel" />{{ t("dpi.verifying", { model: caps.modelName }) }}
          </span>
          <span v-else-if="dev.dpiCheck" class="mt-2 block font-semibold text-wheel" role="status">{{ t(`dpi.check.${dev.dpiCheck.status}`, { model: caps.modelName }) }}</span>
          <details v-if="!dev.verifyingDpi && dev.dpiCheck?.reason" class="mt-1">
            <summary class="w-fit cursor-pointer rounded-sm hover:text-wheel focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-led-deep">{{ t("dpi.details") }}</summary>
            <span class="mt-1 block">{{ dev.dpiCheck.reason }}</span>
          </details>
          <span v-if="!dev.verifyingDpi && !dev.onCableOrReceiver" class="mt-2 block">{{ t("dpi.verifyNeedsLink") }}</span>
        </template>
        <template v-else-if="overLimit">
          <span class="font-semibold text-wheel">{{ t("dpi.overTitle", { dpi: fmt(overLimit) }) }}</span>
          {{ " " }}{{ t("dpi.overBody", { model: caps.modelName, dpi: fmt(overLimit) }) }}
        </template>
      </p>
      <button v-if="!caps.dpiVerified" type="button" :class="[btnGhost, 'ml-auto shrink-0 whitespace-nowrap ring-1 ring-shell-deep']" :disabled="!dev.canVerifyDpi || dev.verifyingDpi" @click="dev.verifyDpi()">
        {{ dev.dpiCheck ? t("dpi.verifyAgain") : t("dpi.verifyNow") }}
      </button>
    </div>

    <div class="grid grid-cols-[180px_minmax(0,1fr)_320px] gap-6 @max-4xl:grid-cols-[minmax(0,1fr)_300px]">
      <div class="@max-4xl:hidden"><MouseSlot class="w-full" /></div>
      <ol id="ladder" :aria-label="t('dpi.stages')" class="flex flex-col gap-1">
        <li v-for="(s, i) in stages" :key="s.index" class="relative" :style="{ '--stage': s.color }">
          <span v-if="i === p.activeStage && isOn(i)" class="flood absolute inset-0 rounded-key" />
          <div :data-row="i" :class="['relative flex h-12 items-center gap-3 rounded-key pl-3 pr-2 transition-colors duration-150 ease-actuation', i === selected ? 'ring-2 ring-wheel' : 'hover:bg-skate/50']">
            <button
              type="button"
              :aria-pressed="i === selected"
              :aria-label="t('dpi.editStage', { n: i + 1 })"
              :class="['flex h-full min-w-0 flex-1 items-center gap-3 rounded-key text-left', focusRing]"
              @click="ui.selection.stage = i"
            >
              <span :class="['lens size-5 shrink-0 rounded-full', !isOn(i) && 'opacity-25 grayscale', i === p.activeStage && isOn(i) && 'lens-glow']" />
              <span class="w-16 whitespace-nowrap text-wheel-muted">{{ t("dpi.stage", { n: i + 1 }) }}</span>
              <span :class="['num text-title font-[430]', !isOn(i) && 'text-wheel-muted/60']">{{ isOn(i) ? fmt(s.dpi) : t("common.off") }}</span>
              <span v-if="i === p.activeStage && isOn(i)" class="ml-1 rounded-full bg-wheel px-2 py-0.5 text-micro font-semibold text-skate">{{ t("dpi.active") }}</span>
            </button>
            <ToggleSwitch :on="isOn(i)" :label="t('dpi.stageOn', { n: i + 1 })" @toggle="toggle(i)" />
          </div>
        </li>
      </ol>
      <DpiStageEditor :index="selected" :on="isOn(selected)" @toggle="toggle(selected)" />
    </div>
  </div>
</template>
