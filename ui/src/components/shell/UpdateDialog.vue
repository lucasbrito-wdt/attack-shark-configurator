<script setup lang="ts">
// The update offer: version, date, notes as plain text, then download progress, install and relaunch.
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { RELEASES_URL, openReleases, useUpdater } from "../../composables/useUpdater";
import { currentLocale } from "../../i18n";
import { btnGhost, btnPrimary, focusRing } from "../../lib/styles";
import { useDevice } from "../../stores/device";

const { t } = useI18n();
const dev = useDevice();
const { state, install } = useUpdater();
const dlg = ref<HTMLDialogElement | null>(null);
const busy = computed(() => state.phase === "downloading" || state.phase === "installing");

watch(
  () => state.open,
  async (open) => {
    await nextTick();
    if (open && !dlg.value?.open) dlg.value?.showModal();
    else if (!open) dlg.value?.close();
  },
);
const close = () => !busy.value && (state.open = false);

const date = computed(() => {
  const d = new Date(state.date);
  return state.date && !Number.isNaN(d.getTime()) ? new Intl.DateTimeFormat(currentLocale(), { dateStyle: "long" }).format(d) : "";
});
const mb = (bytes: number) => new Intl.NumberFormat(currentLocale(), { style: "unit", unit: "megabyte", maximumFractionDigits: 1 }).format(bytes / 1_000_000);
const progress = computed(() => (state.total ? Math.min(1, state.done / state.total) : 0));
</script>

<template>
  <dialog
    ref="dlg"
    class="m-auto w-[460px] rounded-panel bg-skate p-0 text-wheel shadow-float backdrop:bg-wheel/30 backdrop:backdrop-blur-sm"
    aria-labelledby="upd-title"
    @cancel.prevent="close"
  >
    <div v-if="state.open" class="p-6">
      <h2 id="upd-title" class="wide text-title font-[450]">{{ t("update.title", { version: state.version }) }}</h2>
      <p class="mt-1 text-micro text-wheel-muted">
        {{ date ? t("update.releasedOn", { date, current: state.currentVersion }) : t("update.current", { current: state.currentVersion }) }}
      </p>
      <!-- notes are plain text: interpolation escapes them, line breaks are kept by CSS -->
      <p v-if="state.notes" class="mt-4 max-h-48 overflow-y-auto whitespace-pre-line rounded-key bg-shell p-4 text-wheel-muted">{{ state.notes }}</p>

      <div v-if="state.phase === 'downloading'" class="mt-4" role="status">
        <div class="h-2 overflow-hidden rounded-full bg-shell" role="progressbar" :aria-valuenow="Math.round(progress * 100)" aria-valuemin="0" aria-valuemax="100" :aria-label="t('update.downloading')">
          <div class="h-full rounded-full bg-wheel transition-[width] duration-150" :style="{ width: `${state.total ? progress * 100 : 30}%` }" />
        </div>
        <p class="num mt-2 text-micro text-wheel-muted">
          {{ state.total ? t("update.progress", { done: mb(state.done), total: mb(state.total) }) : t("update.progressUnknown", { done: mb(state.done) }) }}
        </p>
      </div>
      <p v-else-if="state.phase === 'installing'" class="mt-4 flex items-center gap-2 font-semibold" role="status">
        <span class="spin size-4 rounded-full border-2 border-shell-deep border-t-wheel" />{{ t("update.installing") }}
      </p>

      <div v-if="state.error" class="mt-4 rounded-key bg-shell p-4" role="alert">
        <p class="font-semibold">{{ t(`update.error.${state.error}`) }}</p>
        <a
          v-if="state.error === 'package'"
          :href="RELEASES_URL"
          :class="['mt-1 inline-block text-led-deep underline', focusRing]"
          @click.prevent="openReleases"
          >{{ t("update.releasesLink") }}</a
        >
      </div>
      <p v-else-if="dev.hasPending && !busy" class="mt-4 text-micro text-wheel-muted">{{ t("update.pendingSaved") }}</p>

      <div class="mt-6 flex justify-end gap-2">
        <button type="button" :class="btnGhost" :disabled="busy" @click="close">{{ t("update.later") }}</button>
        <button v-if="state.error !== 'package'" type="button" :class="btnPrimary" :disabled="busy" @click="install">
          {{ state.error ? t("update.retry") : t("update.now") }}
        </button>
      </div>
    </div>
  </dialog>
</template>
