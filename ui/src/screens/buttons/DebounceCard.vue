<script setup lang="ts">
// Key response time. Hidden when the model has no debounce setting; ms per raw unit differs per platform.
import { computed } from "vue";
import RailInput from "../../components/ui/RailInput.vue";
import { useI18n } from "vue-i18n";
import { useDevice } from "../../stores/device";

const dev = useDevice();
const { t } = useI18n();
const p = computed(() => dev.profile!);
const caps = computed(() => dev.caps!);
</script>

<template>
  <div v-if="caps.debounce && p.debounce != null" class="rounded-panel bg-skate p-4 shadow-lift ring-1 ring-shell-deep">
    <div class="flex items-baseline justify-between gap-3">
      <h2 class="font-semibold">{{ t("buttons.debounce") }}</h2>
      <span class="num shrink-0 whitespace-nowrap font-semibold">{{ p.debounce * caps.debounceMsPerUnit }} ms</span>
    </div>
    <div class="mt-2">
      <RailInput :min="caps.debounce.min" :max="caps.debounce.max" :value="p.debounce" :label="t('buttons.debounce')" @input="(v) => dev.update({ debounce: v }, true)" @commit="() => dev.flush()" />
    </div>
    <p class="text-micro text-wheel-muted">{{ t("buttons.debounceHint") }}</p>
  </div>
</template>
