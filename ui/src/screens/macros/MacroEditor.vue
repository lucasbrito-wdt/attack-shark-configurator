<script setup lang="ts">
// Edits one library macro: record from the keyboard, insert keys and pauses, reorder, set how it plays.
// Edits are saved to the library after a short pause; buttons hold their own copy until they are bound again.
import { computed, onBeforeUnmount, ref, toRaw, watch } from "vue";
import type { LoopMode, Macro, MacroEvent } from "../../api/client";
import AppIcon from "../../components/ui/AppIcon.vue";
import SegmentedControl from "../../components/ui/SegmentedControl.vue";
import { useI18n } from "vue-i18n";
import { friendlyKey, hidFromCode } from "../../lib/hid";
import { buttonName } from "../../lib/labels";
import { btnBase, btnGhost, btnIcon, btnPrimary, fieldNumber, focusRing } from "../../lib/styles";
import { useDevice } from "../../stores/device";
import { useUi } from "../../stores/ui";
import { MAX_DELAY_MS, slotsUsed } from "./slots";

const props = defineProps<{ macro: Macro }>();
const emit = defineEmits<{ remove: [] }>();
const dev = useDevice();
const ui = useUi();
const { t } = useI18n();

const draft = ref<Macro>(structuredClone(toRaw(props.macro)));
const selected = ref<number | null>(null);
watch(() => props.macro.id, () => {
  stopAll();
  draft.value = structuredClone(toRaw(props.macro));
  selected.value = null;
});

const max = computed(() => dev.caps?.maxMacroEvents ?? 50);
const used = computed(() => slotsUsed(draft.value.events));
const keyName = (code: number) => friendlyKey(dev.catalog?.keys.find((k) => k.code === code)?.label ?? t("buttons.key", { code }));
const boundTo = computed(() => {
  const names = new Map(dev.caps?.buttons.map((b) => [b.id, buttonName(b)]));
  return (dev.profile?.buttons ?? []).filter((b) => b.action.kind === "macro" && b.action.macroId === props.macro.id).map((b) => names.get(b.button) ?? t("macros.buttonN", { n: b.button + 1 }));
});
const stale = computed(() => dev.staleMacros.includes(props.macro.id));
const marks = computed(() => {
  const ev = draft.value.events;
  const total = ev.reduce((a, e) => a + e.delayMs, 0) || 1;
  let at = 0;
  return ev.map((e) => {
    const left = `${(at / total) * 100}%`;
    at += e.delayMs;
    return { left, down: e.down };
  });
});

// ---- saving -----------------------------------------------------------------------------
let timer: ReturnType<typeof setTimeout> | null = null;
function save(now = false) {
  if (timer) clearTimeout(timer);
  const run = () => {
    const d = draft.value;
    if (!d.name.trim()) return;
    void dev.updateMacro(d.id, { name: d.name, events: d.events.map((e) => ({ ...e })), loopMode: d.loopMode, loopCount: d.loopCount });
  };
  if (now) run();
  else timer = setTimeout(run, 400);
}
const edit = (fn: (ev: MacroEvent[]) => void) => {
  fn(draft.value.events);
  save();
};

// ---- recording and key insertion share one capture listener -----------------------------
const recording = ref(false);
const awaitKey = ref(false);
let last = 0;
const held = new Set<string>();

function onKey(e: KeyboardEvent) {
  if (e.repeat) return e.preventDefault();
  e.preventDefault();
  e.stopPropagation();
  if (e.type === "keydown" && e.code === "Escape") return stopAll(true);
  const key = hidFromCode(e.code);
  if (awaitKey.value) {
    if (e.type !== "keydown") return;
    awaitKey.value = false;
    release();
    if (key == null) return ui.toast("error", t("buttons.keyNotSendable"));
    const at = selected.value == null ? draft.value.events.length : selected.value + 1;
    edit((ev) => ev.splice(at, 0, { key, down: true, delayMs: 30 }, { key, down: false, delayMs: 40 }));
    selected.value = at;
    return;
  }
  if (key == null) return;
  if (e.type === "keydown") held.add(e.code);
  else if (!held.delete(e.code)) return;
  if (slotsUsed(draft.value.events) >= max.value) {
    stopAll(true);
    return ui.toast("error", t("macros.full", { max: max.value }));
  }
  const now = performance.now();
  const ev = draft.value.events;
  // delayMs is the pause after an event, so the gap lands on the previous one
  if (ev.length) ev[ev.length - 1].delayMs = Math.min(MAX_DELAY_MS, Math.max(10, Math.round(now - last)));
  ev.push({ key, down: e.type === "keydown", delayMs: 20 });
  last = now;
}

function grab() {
  ui.keyboardBusy = true;
  window.addEventListener("keydown", onKey, true);
  window.addEventListener("keyup", onKey, true);
}
function release() {
  ui.keyboardBusy = false;
  window.removeEventListener("keydown", onKey, true);
  window.removeEventListener("keyup", onKey, true);
}
function startRecording() {
  draft.value.events = [];
  selected.value = null;
  held.clear();
  last = performance.now();
  recording.value = true;
  grab();
}
function stopAll(saveNow = false) {
  const wasRecording = recording.value;
  recording.value = false;
  awaitKey.value = false;
  release();
  if (wasRecording && saveNow) save(true);
}
function addKey() {
  awaitKey.value = true;
  grab();
}
onBeforeUnmount(() => {
  stopAll();
  if (timer) {
    clearTimeout(timer);
    save(true);
  }
});

// ---- event list -------------------------------------------------------------------------
function addPause() {
  const i = selected.value ?? draft.value.events.length - 1;
  edit((ev) => (ev[i].delayMs = Math.min(MAX_DELAY_MS, ev[i].delayMs + 50)));
}
function move(d: -1 | 1) {
  const i = selected.value;
  if (i == null) return;
  edit((ev) => ev.splice(i + d, 0, ...ev.splice(i, 1)));
  selected.value = i + d;
}
function removeEvent() {
  const i = selected.value;
  if (i == null) return;
  edit((ev) => ev.splice(i, 1));
  selected.value = draft.value.events.length ? Math.min(i, draft.value.events.length - 1) : null;
}
function setDelay(i: number, v: string) {
  const n = Math.round(Number(v));
  if (Number.isFinite(n) && n >= 1) edit((ev) => (ev[i].delayMs = Math.min(MAX_DELAY_MS, n)));
}

const LOOP_OPTIONS = computed<{ value: LoopMode; label: string }[]>(() => [
  { value: "count", label: t("macros.loop.count") },
  { value: "untilKey", label: t("macros.loop.untilKey") },
  { value: "whileHeld", label: t("macros.loop.whileHeld") },
]);
function setLoop(mode: LoopMode) {
  draft.value.loopMode = mode;
  save();
}
function setCount(v: string) {
  const n = Math.round(Number(v));
  if (n >= 1 && n <= 255) {
    draft.value.loopCount = n;
    save();
  }
}
</script>

<template>
  <section :aria-label="t('macros.editor')" class="rounded-panel bg-skate p-6 shadow-lift ring-1 ring-shell-deep">
    <div class="flex items-center gap-3">
      <label class="min-w-0 flex-1">
        <span class="sr-only">{{ t("macros.name") }}</span>
        <input v-model="draft.name" type="text" maxlength="48" :class="['wide w-full rounded-key bg-transparent px-1 text-title font-[450] outline-none hover:bg-shell/60', focusRing]" @input="save()" @blur="!draft.name.trim() && (draft.name = macro.name)" />
      </label>
      <button v-if="recording" type="button" :class="[btnBase, 'h-10 bg-sensor px-4 text-skate shadow-lift hover:bg-sensor/90 active:shadow-press']" @click="stopAll(true)">
        <span class="rec"><AppIcon name="stop" class="size-4" /></span>{{ t("macros.stop") }}
      </button>
      <button v-else type="button" :class="btnPrimary" :disabled="awaitKey" @click="startRecording">
        <span class="text-sensor"><AppIcon name="record" class="size-4" /></span>{{ t("macros.record") }}
      </button>
      <button type="button" :aria-label="t('macros.delete')" :class="btnIcon" :disabled="recording" @click="emit('remove')"><AppIcon name="trash" class="size-[18px]" /></button>
    </div>
    <div class="mt-3 flex items-center justify-between gap-4 text-micro text-wheel-muted" aria-live="polite">
      <span v-if="recording" class="font-semibold text-sensor">{{ t("macros.recording") }}</span>
      <span v-else-if="stale" class="font-semibold text-led-deep">{{ t("macros.staleHint", { buttons: boundTo.join(", ") }) }}</span>
      <span v-else>{{ boundTo.length ? t("macros.boundTo", { buttons: boundTo.join(", ") }) : t("macros.notBound") }}</span>
      <span :class="['num shrink-0', used > max - 6 && 'font-semibold text-sensor']">{{ t("macros.slots", { used, max }) }}</span>
    </div>
    <div class="relative mt-3 h-8 rounded-[7px] bg-shell px-2" aria-hidden="true">
      <div class="relative h-full">
        <span v-for="(m, i) in marks" :key="i" :class="['absolute bottom-0 w-0.5 rounded-full', i === selected ? 'h-full bg-led' : m.down ? 'h-full bg-wheel' : 'h-1/2 bg-wheel-muted/60']" :style="{ left: m.left }" />
      </div>
    </div>
    <ol :aria-label="t('macros.events')" class="mt-3 flex max-h-[min(300px,calc(100dvh-470px))] min-h-24 flex-col gap-0.5 overflow-y-auto">
      <li v-if="!draft.events.length" class="grid h-24 place-items-center rounded-key border border-dashed border-shell-deep text-wheel-muted">{{ t("macros.noEvents") }}</li>
      <li v-for="(e, i) in draft.events" :key="i" :class="['grid grid-cols-[32px_28px_minmax(0,1fr)_150px] items-center gap-3 rounded-key px-3 py-1', i === selected ? 'bg-shell ring-1 ring-wheel/30' : 'hover:bg-shell/60']">
        <span class="num text-micro text-wheel-muted">{{ i + 1 }}</span>
        <span :class="e.down ? 'text-wheel' : 'text-wheel-muted'" :title="e.down ? t('macros.pressedTitle') : t('macros.releasedTitle')"><AppIcon :name="e.down ? 'pressed' : 'released'" class="size-4" /></span>
        <button type="button" :aria-pressed="i === selected" :class="['flex items-center gap-2 rounded-[6px] text-left transition-colors hover:bg-shell/60', focusRing]" @click="selected = i">
          <kbd class="min-w-7 rounded-[6px] bg-skate px-2 py-0.5 text-center font-sans font-semibold shadow-lift ring-1 ring-shell-deep">{{ keyName(e.key) }}</kbd>
          <span class="text-wheel-muted">{{ e.down ? t("macros.pressed") : t("macros.released") }}</span>
        </button>
        <label class="flex items-center justify-end gap-2 text-micro text-wheel-muted">
          <span>{{ t("macros.then") }}</span>
          <span class="sr-only">{{ t("macros.pauseAfter", { n: i + 1 }) }}</span>
          <input type="number" min="1" :max="MAX_DELAY_MS" :value="e.delayMs" :disabled="recording" class="num h-8 w-20 rounded-[7px] bg-skate px-2 text-right text-body text-wheel ring-1 ring-shell-deep focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-led-deep" @change="setDelay(i, ($event.target as HTMLInputElement).value)" />ms
        </label>
      </li>
    </ol>
    <div class="mt-3 flex flex-wrap items-center gap-1 border-t border-shell-deep pt-3">
      <button type="button" :class="btnGhost" :disabled="recording || used >= max - 1" :aria-pressed="awaitKey" @click="awaitKey ? stopAll() : addKey()">
        <AppIcon name="key" class="size-4" />{{ awaitKey ? t("macros.pressKey") : t("macros.addKey") }}
      </button>
      <button type="button" :class="btnGhost" :disabled="!draft.events.length || recording" @click="addPause"><AppIcon name="clock" class="size-4" />{{ t("macros.addPause") }}</button>
      <span class="mx-1 h-5 w-px bg-shell-deep" />
      <button type="button" :aria-label="t('macros.moveUp')" :class="btnIcon" :disabled="selected == null || selected === 0 || recording" @click="move(-1)"><AppIcon name="up" class="size-4" /></button>
      <button type="button" :aria-label="t('macros.moveDown')" :class="btnIcon" :disabled="selected == null || selected >= draft.events.length - 1 || recording" @click="move(1)"><AppIcon name="down" class="size-4" /></button>
      <button type="button" :class="btnGhost" :disabled="selected == null || recording" @click="removeEvent">{{ t("macros.deleteEvent") }}</button>
      <button type="button" :class="[btnGhost, 'ml-auto']" :disabled="!draft.events.length || recording" @click="edit((ev) => ev.splice(0)); selected = null">{{ t("macros.clearAll") }}</button>
    </div>
    <div class="mt-4 flex flex-wrap items-center gap-3">
      <span class="text-micro text-wheel-muted">{{ t("macros.whenPressed") }}</span>
      <SegmentedControl :name="t('macros.playback')" :options="LOOP_OPTIONS" :value="draft.loopMode" @select="setLoop" />
      <label v-if="draft.loopMode === 'count'" class="flex items-center gap-2">
        <span class="sr-only">{{ t("macros.timesToPlay") }}</span>
        <input type="number" min="1" max="255" :value="draft.loopCount" :class="[fieldNumber, 'w-16']" @change="setCount(($event.target as HTMLInputElement).value)" />
        <span class="text-wheel-muted">{{ t("macros.times", draft.loopCount) }}</span>
      </label>
    </div>
  </section>
</template>
