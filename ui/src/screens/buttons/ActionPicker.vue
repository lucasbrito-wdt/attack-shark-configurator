<script setup lang="ts">
// What one button does: preset actions by category, a captured key combination, or a macro from the library.
import { computed, onBeforeUnmount, ref, watch } from "vue";
import type { ActionRef, ButtonBinding, ButtonInfo } from "../../api/client";
import AppIcon from "../../components/ui/AppIcon.vue";
import { useI18n } from "vue-i18n";
import { actionName, bindingName, buttonName, categoryName } from "../../lib/labels";
import { IS_MAC, friendlyKey, hidFromCode, isModifierCode, modLabels, modsFromEvent } from "../../lib/hid";
import { btnGhost, focusRing } from "../../lib/styles";
import { useDevice } from "../../stores/device";
import { useUi } from "../../stores/ui";

const props = defineProps<{ info: ButtonInfo; binding: ButtonBinding }>();
const emit = defineEmits<{ bind: [action: ActionRef] }>();
const dev = useDevice();
const ui = useUi();
const { t, te } = useI18n();

type TabId = "mouse" | "dpi" | "media" | "browser" | "function" | "keyboard" | "macro";

const tabs = computed(() => {
  const list: { id: TabId; name: string }[] = (dev.catalog?.categories ?? []).map((c) => ({
    id: c.id === "shortcut" ? "keyboard" : c.id,
    name: categoryName(c.id, c.label),
  }));
  if (dev.caps?.supportsMacros) list.push({ id: "macro", name: t("action.category.macro") });
  return list;
});

function tabOf(a: ActionRef): TabId {
  if (a.kind === "shortcut") return "keyboard";
  if (a.kind === "macro") return dev.caps?.supportsMacros ? "macro" : "mouse";
  const c = dev.catalog?.categories.find((x) => x.actions.some((e) => e.id === a.id));
  return !c ? "mouse" : c.id === "shortcut" ? "keyboard" : c.id;
}

const tab = ref<TabId>(tabOf(props.binding.action));
watch(() => props.binding.button, () => (tab.value = tabOf(props.binding.action)));

const actions = computed(() => {
  const id = tab.value === "keyboard" ? "shortcut" : tab.value;
  return dev.catalog?.categories.find((c) => c.id === id)?.actions ?? [];
});
const a = computed(() => props.binding.action);
const isPreset = (id: number) => a.value.kind === "preset" && a.value.id === id;
const isMacro = (id: number) => a.value.kind === "macro" && a.value.macroId === id;

// ---- key combination capture ------------------------------------------------------------
const capturing = ref(false);
const liveMods = ref(0);
const keyName = (code: number) => friendlyKey(dev.catalog?.keys.find((k) => k.code === code)?.label ?? t("buttons.key", { code }));
const comboLabel = (key: number, mods: number) => [...modLabels(mods), keyName(key)].join(" + ");

function onCaptureKey(e: KeyboardEvent) {
  e.preventDefault();
  e.stopPropagation();
  if (e.code === "Escape") return stop();
  liveMods.value = modsFromEvent(e);
  if (isModifierCode(e.code) || e.repeat) return;
  const key = hidFromCode(e.code);
  if (key == null) return ui.toast("error", t("buttons.keyNotSendable"));
  stop();
  emit("bind", { kind: "shortcut", key, mods: modsFromEvent(e) });
}
const onCaptureUp = (e: KeyboardEvent) => (liveMods.value = modsFromEvent(e));

function start() {
  capturing.value = true;
  liveMods.value = 0;
  ui.keyboardBusy = true;
  window.addEventListener("keydown", onCaptureKey, true);
  window.addEventListener("keyup", onCaptureUp, true);
}
function stop() {
  capturing.value = false;
  ui.keyboardBusy = false;
  window.removeEventListener("keydown", onCaptureKey, true);
  window.removeEventListener("keyup", onCaptureUp, true);
}
watch(() => props.binding.button, stop);
onBeforeUnmount(stop);

/** Radio-group arrow keys over a list of buttons that are laid out in a grid. */
function onRadioKey(e: KeyboardEvent) {
  const d = e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : e.key === "ArrowLeft" || e.key === "ArrowUp" ? -1 : 0;
  if (!d) return;
  e.preventDefault();
  const items = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[role="radio"]')];
  const i = items.indexOf(document.activeElement as HTMLElement);
  items[(i + d + items.length) % items.length]?.focus();
}
function onTabKey(e: KeyboardEvent) {
  const d = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
  if (!d) return;
  e.preventDefault();
  const i = tabs.value.findIndex((x) => x.id === tab.value);
  tab.value = tabs.value[(i + d + tabs.value.length) % tabs.value.length].id;
  const list = (e.currentTarget as HTMLElement).parentElement;
  requestAnimationFrame(() => (list?.querySelector('[aria-selected="true"]') as HTMLElement | null)?.focus());
}
</script>

<template>
  <section :aria-label="t('buttons.actionOf', { button: buttonName(info) })" class="self-start rounded-panel bg-skate p-6 shadow-lift ring-1 ring-shell-deep">
    <div class="flex items-start justify-between gap-4">
      <div>
        <p class="text-micro text-wheel-muted">{{ te(`buttons.place.${info.position}`) ? t(`buttons.place.${info.position}`) : t("buttons.place.extra") }}</p>
        <h2 class="wide text-title font-[450]">{{ buttonName(info) }}</h2>
      </div>
      <span class="max-w-[50%] truncate rounded-full bg-shell px-3 py-1 text-micro font-semibold">{{ bindingName(binding) }}</span>
    </div>

    <div role="tablist" :aria-label="t('buttons.actionType')" class="mt-4 flex flex-wrap gap-1">
      <button
        v-for="tb in tabs"
        :key="tb.id"
        type="button"
        role="tab"
        :aria-selected="tb.id === tab"
        :tabindex="tb.id === tab ? 0 : -1"
        :class="['h-8 shrink-0 rounded-full px-3 text-micro font-semibold transition-colors duration-150', tb.id === tab ? 'bg-wheel text-skate' : 'text-wheel-muted hover:bg-shell hover:text-wheel', focusRing]"
        @click="tab = tb.id"
        @keydown="onTabKey"
      >
        {{ tb.name }}
      </button>
    </div>

    <div role="tabpanel" :aria-label="tabs.find((x) => x.id === tab)?.name">
      <div v-if="tab === 'macro'" class="mt-4">
        <div v-if="dev.macros.length" role="radiogroup" :aria-label="t('action.category.macro')" class="flex flex-col gap-1" @keydown="onRadioKey">
          <button
            v-for="m in dev.macros"
            :key="m.id"
            type="button"
            role="radio"
            :aria-checked="isMacro(m.id)"
            :class="['flex items-center gap-3 rounded-key px-3 py-2 text-left ring-1 transition-colors', isMacro(m.id) ? 'bg-shell ring-wheel' : 'ring-shell-deep hover:bg-shell', focusRing]"
            @click="emit('bind', { kind: 'macro', macroId: m.id })"
          >
            <AppIcon name="macros" class="size-4 text-wheel-muted" />
            <span class="flex-1 truncate font-semibold">{{ m.name }}</span>
            <span class="text-micro text-wheel-muted">{{ t("buttons.events", { n: m.events.length }, m.events.length) }}</span>
          </button>
        </div>
        <p v-else class="rounded-key bg-shell p-4 text-wheel-muted">{{ t("buttons.noMacros") }}</p>
        <button type="button" :class="[btnGhost, 'mt-2 px-0 text-led-deep hover:bg-transparent hover:underline']" @click="ui.go('macros')">{{ t("buttons.openMacros") }}</button>
      </div>

      <template v-else>
        <div v-if="tab === 'keyboard'" class="mt-4 rounded-key bg-shell p-4">
          <div class="flex items-center justify-between gap-3">
            <span class="font-semibold">{{ t("buttons.customCombo") }}</span>
            <span class="text-micro text-wheel-muted">{{ IS_MAC ? t("buttons.cmdAsWin") : t("buttons.modsSupported") }}</span>
          </div>
          <button
            type="button"
            aria-live="polite"
            :aria-pressed="capturing"
            :class="['mt-3 flex h-12 w-full items-center justify-center gap-2 rounded-key bg-skate text-lead ring-1 transition-[box-shadow]', capturing ? 'ring-2 ring-led' : 'ring-shell-deep hover:ring-wheel-muted', focusRing]"
            @click="capturing ? stop() : start()"
            @blur="stop"
          >
            <span v-if="capturing" class="text-wheel-muted">{{ liveMods ? `${modLabels(liveMods).join(" + ")} + ...` : t("buttons.pressNow") }}</span>
            <template v-else-if="a.kind === 'shortcut'">
              <span class="font-semibold">{{ comboLabel(a.key, a.mods) }}</span><span class="text-micro text-wheel-muted">{{ t("buttons.clickToChange") }}</span>
            </template>
            <template v-else><AppIcon name="key" class="size-5 text-wheel-muted" /><span class="text-wheel-muted">{{ t("buttons.clickThenPress") }}</span></template>
          </button>
        </div>
        <div role="radiogroup" :aria-label="t('buttons.actionsOf', { tab: tabs.find((x) => x.id === tab)?.name ?? '' })" class="mt-4 grid grid-cols-2 gap-2" @keydown="onRadioKey">
          <button
            v-for="x in actions"
            :key="x.id"
            type="button"
            role="radio"
            :aria-checked="isPreset(x.id)"
            :class="[
              'flex h-10 min-w-0 items-center gap-2 rounded-key px-3 text-left ring-1 transition-[background-color,box-shadow] duration-150 ease-actuation active:scale-[0.98]',
              isPreset(x.id) ? 'bg-wheel text-skate ring-wheel' : 'bg-skate ring-shell-deep hover:ring-wheel-muted/60',
              focusRing,
            ]"
            @click="emit('bind', { kind: 'preset', id: x.id })"
          >
            <span class="truncate">{{ actionName(x.id, x.label) }}</span>
            <AppIcon v-if="isPreset(x.id)" name="check" class="ml-auto size-4" />
          </button>
        </div>
      </template>
    </div>
  </section>
</template>
