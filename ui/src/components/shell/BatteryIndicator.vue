<script setup lang="ts">
// Always-visible battery in the shell: live level, else the last reading marked stale, else "--". Never a guess.
import { useI18n } from "vue-i18n";
import { useBatteryBadge } from "../../composables/useBattery";
import { focusRing } from "../../lib/styles";
import { useUi } from "../../stores/ui";
import BatteryMeter from "../ui/BatteryMeter.vue";

const ui = useUi();
const { t } = useI18n();
const badge = useBatteryBadge();
</script>

<template>
  <div class="group relative">
    <button
      type="button"
      :aria-label="badge.label"
      :class="['flex w-full flex-col gap-1.5 rounded-key px-3 py-2 text-left transition-colors duration-150 ease-actuation hover:bg-shell/80', focusRing]"
      @click="ui.go('power')"
    >
      <span class="flex w-full items-baseline gap-2">
        <span class="text-micro text-wheel-muted">{{ t("battery.title") }}</span>
        <span
          :class="[
            'num ml-auto text-body font-semibold',
            badge.low ? 'text-sensor' : badge.level == null ? 'text-wheel-muted' : 'text-wheel',
            badge.stale && 'opacity-55',
          ]"
        >{{ badge.value }}</span>
      </span>
      <BatteryMeter :level="badge.level" :charging="badge.charging" :class="['h-1.5 w-full', badge.stale && 'opacity-45']" />
      <span :class="['line-clamp-2 text-micro', badge.charging ? 'text-led-deep' : 'text-wheel-muted']">{{ badge.detail }}</span>
    </button>
    <p
      v-if="badge.tip"
      aria-hidden="true"
      class="pointer-events-none absolute inset-x-0 bottom-full mb-2 rounded-key bg-wheel px-3 py-2 text-micro text-skate opacity-0 shadow-float transition-opacity duration-150 ease-actuation group-hover:opacity-100 group-has-[:focus-visible]:opacity-100"
    >
      {{ badge.tip }}
    </p>
  </div>
</template>
