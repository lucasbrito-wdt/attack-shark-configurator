<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import AppIcon from "../components/ui/AppIcon.vue";
import ScreenHead from "../components/ui/ScreenHead.vue";
import { btnSecondary, focusRing } from "../lib/styles";
import { useDevice } from "../stores/device";
import { useUi } from "../stores/ui";
import MacroEditor from "./macros/MacroEditor.vue";
import { slotsUsed } from "./macros/slots";

const dev = useDevice();
const ui = useUi();
const { t } = useI18n();
const max = computed(() => dev.caps?.maxMacroEvents ?? 50);
const current = computed(() => dev.macros.find((m) => m.id === ui.selection.macro) ?? dev.macros[0] ?? null);

async function add() {
  const base = t("macros.defaultName");
  let name = base;
  for (let k = 2; dev.macros.some((m) => m.name === name); k++) name = `${base} ${k}`;
  if (await dev.createMacro({ name, events: [], loopMode: "count", loopCount: 1 })) ui.selection.macro = dev.macros[dev.macros.length - 1].id;
}

async function remove() {
  const m = current.value;
  if (!m) return;
  const bound = dev.profile?.buttons.some((b) => b.action.kind === "macro" && b.action.macroId === m.id);
  const ok = await ui.confirm({
    title: t("macros.deleteTitle", { name: m.name }),
    body: bound ? t("macros.deleteBound") : t("macros.deleteBody"),
    ok: t("macros.delete"),
    danger: true,
  });
  if (ok && (await dev.deleteMacro(m.id))) ui.selection.macro = null;
}
</script>

<template>
  <div>
    <ScreenHead :title="t('nav.macros')" :sub="t('macros.sub')" />
    <div v-if="dev.caps && !dev.caps.supportsMacros" class="grid max-w-[640px] gap-3 rounded-panel bg-skate/60 p-8 ring-1 ring-shell-deep">
      <AppIcon name="macros" class="size-6 text-wheel-muted" />
      <h2 class="font-semibold">{{ t("macros.unsupportedTitle", { model: dev.caps.modelName }) }}</h2>
      <p class="text-wheel-muted">{{ t("macros.unsupportedBody") }}</p>
      <button type="button" :class="[btnSecondary, 'mt-2 justify-self-start']" @click="ui.go('buttons')">{{ t("macros.openButtons") }}</button>
    </div>
    <div v-else class="grid grid-cols-[240px_minmax(0,1fr)] gap-6 @max-4xl:grid-cols-[200px_minmax(0,1fr)]">
      <aside :aria-label="t('macros.library')" class="flex flex-col gap-2">
        <ul class="flex flex-col gap-1">
          <li v-if="!dev.macros.length" class="rounded-key bg-shell p-4 text-wheel-muted">{{ t("macros.none") }}</li>
          <li v-for="m in dev.macros" :key="m.id">
            <button
              type="button"
              :aria-pressed="m.id === current?.id"
              :class="['flex w-full items-center gap-3 rounded-key px-3 py-2 text-left transition-colors', m.id === current?.id ? 'bg-skate shadow-lift ring-1 ring-shell-deep' : 'hover:bg-skate/60', focusRing]"
              @click="ui.selection.macro = m.id"
            >
              <span class="min-w-0 flex-1">
                <span class="block truncate font-semibold">{{ m.name }}</span>
                <span class="block text-micro text-wheel-muted">{{ t("macros.eventsSlots", { events: t("buttons.events", { n: m.events.length }, m.events.length), used: slotsUsed(m.events), max }) }}</span>
              </span>
              <span v-if="dev.staleMacros.includes(m.id)" class="size-2 shrink-0 rounded-full bg-led" :title="t('macros.editedAfter')" />
            </button>
          </li>
        </ul>
        <button type="button" :class="[btnSecondary, 'mt-1']" @click="add"><AppIcon name="plus" class="size-4" />{{ t("macros.new") }}</button>
      </aside>
      <MacroEditor v-if="current" :macro="current" @remove="remove" />
      <div v-else class="grid place-items-center rounded-panel bg-skate/60 p-12 text-center text-wheel-muted ring-1 ring-shell-deep">{{ t("macros.createToStart") }}</div>
    </div>
  </div>
</template>
