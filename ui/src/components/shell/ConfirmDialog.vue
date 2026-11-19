<script setup lang="ts">
// Native <dialog>: focus trap, Esc and inert background for free.
import { nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { btnDanger, btnGhost, btnPrimary } from "../../lib/styles";
import { useUi } from "../../stores/ui";

const ui = useUi();
const { t } = useI18n();
const dlg = ref<HTMLDialogElement | null>(null);
const field = ref<HTMLInputElement | null>(null);
const value = ref("");
const error = ref<string | null>(null);

watch(
  () => ui.confirmReq,
  async (r) => {
    if (!r) return;
    value.value = r.input?.value ?? "";
    error.value = null;
    await nextTick();
    dlg.value?.showModal();
    field.value?.select();
  },
);

function finish(result: string | boolean | null) {
  const r = ui.confirmReq;
  ui.confirmReq = null;
  dlg.value?.close();
  r?.resolve(result);
}

function submit() {
  const r = ui.confirmReq;
  if (!r) return;
  if (r.input) {
    error.value = r.input.validate(value.value);
    if (error.value) return;
    return finish(value.value);
  }
  finish(true);
}
</script>

<template>
  <dialog
    ref="dlg"
    class="m-auto w-[420px] rounded-panel bg-skate p-0 text-wheel shadow-float backdrop:bg-wheel/30 backdrop:backdrop-blur-sm"
    aria-labelledby="dlg-title"
    @cancel.prevent="finish(null)"
  >
    <form v-if="ui.confirmReq" class="p-6" @submit.prevent="submit">
      <h2 id="dlg-title" class="wide text-title font-[450]">{{ ui.confirmReq.title }}</h2>
      <p v-if="ui.confirmReq.body" class="mt-2 text-wheel-muted">{{ ui.confirmReq.body }}</p>
      <template v-if="ui.confirmReq.input">
        <label class="mt-4 block">
          <span class="text-micro text-wheel-muted">{{ ui.confirmReq.input.label }}</span>
          <input
            ref="field"
            v-model="value"
            :maxlength="ui.confirmReq.input.maxlength"
            required
            :aria-invalid="!!error"
            aria-describedby="dlg-err"
            class="mt-1 h-10 w-full rounded-key bg-shell px-3 ring-1 ring-shell-deep focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-led-deep"
          />
        </label>
        <p id="dlg-err" :class="['mt-2 text-micro font-semibold text-sensor', !error && 'hidden']" role="alert">{{ error }}</p>
      </template>
      <div class="mt-6 flex justify-end gap-2">
        <button type="button" :class="btnGhost" @click="finish(null)">{{ t("common.cancel") }}</button>
        <button type="submit" :class="ui.confirmReq.danger ? btnDanger : btnPrimary">{{ ui.confirmReq.ok }}</button>
      </div>
    </form>
  </dialog>
</template>
