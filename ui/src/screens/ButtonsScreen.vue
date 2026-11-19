<script setup lang="ts">
import { computed } from "vue";
import type { ActionRef, ButtonPosition } from "../api/client";
import MouseSlot from "../components/stage/MouseSlot.vue";
import AppIcon from "../components/ui/AppIcon.vue";
import ScreenHead from "../components/ui/ScreenHead.vue";
import { useI18n } from "vue-i18n";
import { bindingName, buttonName } from "../lib/labels";
import { orderedButtons } from "../lib/caps";
import { pct } from "../lib/models";
import { btnGhost, focusRing } from "../lib/styles";
import { useDevice } from "../stores/device";
import { useUi } from "../stores/ui";
import ActionPicker from "./buttons/ActionPicker.vue";
import DebounceCard from "./buttons/DebounceCard.vue";

const dev = useDevice();
const ui = useUi();
const { t } = useI18n();
const p = computed(() => dev.profile!);
const caps = computed(() => dev.caps!);
const list = computed(() => orderedButtons(caps.value).map((info, i) => ({ info, n: i + 1, binding: p.value.buttons[info.id] })));
const sel = computed(() => list.value.find((b) => b.info.id === ui.selection.button) ?? list.value[0]);

/** Factory action per position (catalog preset ids). */
const DEFAULT_PRESET: Record<ButtonPosition, number> = { left: 0x01, right: 0x02, middle: 0x03, "side-forward": 0x04, "side-back": 0x05 };
const LEFT_CLICK = 0x01;
const isLeftClick = (a: ActionRef) => a.kind === "preset" && a.id === LEFT_CLICK;

async function bind(button: number, action: ActionRef) {
  const others = list.value.some((b) => b.info.id !== button && isLeftClick(b.binding.action));
  if (isLeftClick(p.value.buttons[button].action) && !isLeftClick(action) && !others) {
    return ui.toast("error", t("buttons.keepLeftClick"));
  }
  await dev.setButton(button, action);
}

async function restoreDefaults() {
  for (const b of list.value) {
    const id = DEFAULT_PRESET[b.info.position];
    if (id != null && !(b.binding.action.kind === "preset" && b.binding.action.id === id)) await dev.setButton(b.info.id, { kind: "preset", id });
  }
  ui.toast("ok", t("buttons.restored"));
}
</script>

<template>
  <div>
    <ScreenHead :title="t('nav.buttons')" :sub="list.length ? t('buttons.sub') : undefined">
      <button v-if="list.length" type="button" :class="btnGhost" @click="restoreDefaults">{{ t("buttons.restore") }}</button>
    </ScreenHead>
    <!-- a model whose button map is not decoded yet: no buttons to bind, the response time still applies -->
    <div v-if="!sel" class="grid max-w-[640px] gap-6">
      <div class="grid gap-3 rounded-panel bg-skate/60 p-8 ring-1 ring-shell-deep">
        <AppIcon name="buttons" class="size-6 text-wheel-muted" />
        <h2 class="font-semibold">{{ t("buttons.unavailableTitle") }}</h2>
        <p class="text-wheel-muted">{{ t("buttons.unavailableBody", { model: caps.modelName }) }}</p>
      </div>
      <DebounceCard class="max-w-[320px]" />
    </div>
    <div v-else class="grid grid-cols-[220px_200px_minmax(0,1fr)] gap-6 @max-4xl:grid-cols-[200px_minmax(0,1fr)]">
      <div class="flex flex-col gap-6">
        <ul :aria-label="t('buttons.list')" class="flex flex-col gap-1">
          <li v-for="b in list" :key="b.info.id">
            <button
              type="button"
              :aria-pressed="b.info.id === sel.info.id"
              :class="['flex w-full items-center gap-3 rounded-key px-3 py-2 text-left transition-colors duration-150', b.info.id === sel.info.id ? 'bg-skate shadow-lift ring-1 ring-shell-deep' : 'hover:bg-skate/60', focusRing]"
              @click="ui.selection.button = b.info.id"
            >
              <span :class="['grid size-7 shrink-0 place-items-center rounded-full text-micro font-bold', b.info.id === sel.info.id ? 'bg-wheel text-skate' : 'bg-shell-deep/70']">{{ b.n }}</span>
              <span class="min-w-0">
                <span class="block truncate font-semibold">{{ buttonName(b.info) }}</span>
                <span class="block truncate text-micro text-wheel-muted">{{ bindingName(b.binding) }}</span>
              </span>
            </button>
          </li>
        </ul>
        <DebounceCard />
      </div>
      <div class="relative @max-4xl:hidden">
        <MouseSlot v-slot="{ art }" class="w-full">
          <template v-for="b in list" :key="b.info.id">
            <button
              v-if="art.spots[b.info.position]"
              type="button"
              :aria-label="t('buttons.numbered', { n: b.n, name: buttonName(b.info) })"
              :aria-pressed="b.info.id === sel.info.id"
              :style="pct(art, art.spots[b.info.position]!)"
              :class="[
                'absolute z-[4] grid size-8 -translate-x-1/2 -translate-y-1/2 place-items-center rounded-full text-micro font-bold transition-[transform,background-color] duration-200 ease-actuation hover:scale-110 active:scale-95',
                b.info.id === sel.info.id ? 'bg-wheel text-skate ring-4 ring-led/40' : 'bg-skate text-wheel shadow-lift ring-1 ring-shell-deep',
                focusRing,
              ]"
              @click="ui.selection.button = b.info.id"
            >
              {{ b.n }}
            </button>
          </template>
        </MouseSlot>
      </div>
      <ActionPicker :info="sel.info" :binding="sel.binding" @bind="(a) => bind(sel.info.id, a)" />
    </div>
  </div>
</template>
