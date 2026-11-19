<script setup lang="ts">
// Profiles are context for everything below, so they sit at the top of the sidebar as a switcher,
// like accounts in a mail app. Destructive entries confirm first; the last profile cannot be deleted.
import { nextTick, onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";
import { pickOpenPath, pickSavePath } from "../../api/client";
import { focusRing } from "../../lib/styles";
import { useDevice } from "../../stores/device";
import { useUi } from "../../stores/ui";
import AppIcon from "../ui/AppIcon.vue";

const dev = useDevice();
const ui = useUi();
const { t } = useI18n();
const open = ref(false);
const btn = ref<HTMLButtonElement | null>(null);
const menu = ref<HTMLElement | null>(null);
const item = `flex w-full items-center gap-2 rounded-[7px] px-3 py-2 text-left hover:bg-shell disabled:cursor-not-allowed disabled:opacity-45 ${focusRing}`;

const items = () => [...(menu.value?.querySelectorAll<HTMLElement>("[role^=menuitem]:not(:disabled)") ?? [])];

async function toggle() {
  open.value = !open.value;
  if (!open.value) return;
  await nextTick();
  (menu.value?.querySelector<HTMLElement>('[aria-checked="true"]') ?? items()[0])?.focus();
}

function close(refocus = true) {
  open.value = false;
  if (refocus) btn.value?.focus();
}

function onMenuKey(e: KeyboardEvent) {
  const list = items();
  const i = list.indexOf(document.activeElement as HTMLElement);
  if (e.key === "Escape") return e.preventDefault(), close();
  if (e.key === "Tab") return close(false);
  const d = e.key === "ArrowDown" ? 1 : e.key === "ArrowUp" ? -1 : 0;
  if (e.key === "Home" || e.key === "End") {
    e.preventDefault();
    list[e.key === "Home" ? 0 : list.length - 1]?.focus();
  } else if (d) {
    e.preventDefault();
    list[(i + d + list.length) % list.length]?.focus();
  }
}

function onDocClick(e: MouseEvent) {
  if (open.value && !(e.target as HTMLElement).closest("[data-profile-switcher]")) open.value = false;
}
document.addEventListener("click", onDocClick);
onBeforeUnmount(() => document.removeEventListener("click", onDocClick));

const validateName = (others: string[]) => (v: string) => {
  const n = v.trim();
  if (!n) return t("profile.nameEmpty");
  if (others.some((o) => o.toLowerCase() === n.toLowerCase())) return t("profile.nameTaken");
  return null;
};

function uniqueName(base: string) {
  let n = base, k = 2;
  while (dev.profiles.some((p) => p.name === n)) n = `${base} ${k++}`;
  return n;
}

async function run(action: string) {
  const p = dev.profile;
  close();
  if (!p) return;
  const others = dev.profiles.filter((x) => x.id !== p.id).map((x) => x.name);
  if (action === "add") await dev.createProfile(uniqueName(t("profile.new")));
  if (action === "rename") {
    const v = await ui.confirm({ title: t("profile.renameTitle"), ok: t("profile.rename"), input: { label: t("profile.name"), value: p.name, maxlength: 48, validate: validateName(others) } });
    if (typeof v === "string") await dev.renameProfile(p.id, v.trim());
  }
  if (action === "import") {
    const path = await pickOpenPath();
    if (path) await dev.importProfile(path);
  }
  if (action === "export") {
    const path = await pickSavePath(p.name);
    if (path) await dev.exportProfile(p.id, path);
  }
  if (action === "reset") {
    const ok = await ui.confirm({ title: t("profile.resetTitle", { name: p.name }), body: t("profile.resetBody"), ok: t("profile.resetOk"), danger: true });
    if (ok) await dev.resetProfile(p.id);
  }
  if (action === "delete") {
    const ok = await ui.confirm({ title: t("profile.deleteTitle", { name: p.name }), body: t("profile.deleteBody"), ok: t("profile.deleteOk"), danger: true });
    if (ok) await dev.deleteProfile(p.id);
  }
}

async function select(id: number) {
  close();
  if (id !== dev.profile?.id) await dev.selectProfile(id);
}
</script>

<template>
  <div data-profile-switcher class="relative px-3">
    <button
      ref="btn"
      type="button"
      aria-haspopup="menu"
      :aria-expanded="open"
      :class="['flex w-full items-center gap-3 rounded-key bg-shell/70 px-3 py-2 text-left ring-1 ring-shell-deep transition-colors duration-150 hover:bg-shell active:shadow-press', focusRing]"
      @click="toggle"
    >
      <span class="min-w-0 flex-1">
        <span class="block text-micro text-wheel-muted">{{ t("profile.label") }}</span>
        <span class="block truncate font-semibold">{{ dev.profile?.name ?? t("profile.loading") }}</span>
      </span>
      <AppIcon name="chevron" :class="['size-4 text-wheel-muted transition-transform duration-200 ease-actuation', open && 'rotate-180']" />
    </button>
    <div v-if="open" ref="menu" role="menu" :aria-label="t('profile.menu')" class="absolute left-3 right-3 top-full z-50 mt-2 rounded-key bg-skate p-1 shadow-float ring-1 ring-shell-deep" @keydown="onMenuKey">
      <button v-for="p in dev.profiles" :key="p.id" type="button" role="menuitemradio" :aria-checked="p.id === dev.profile?.id" :class="item" @click="select(p.id)">
        <span class="w-4"><AppIcon v-if="p.id === dev.profile?.id" name="check" class="size-4 text-led-deep" /></span>
        <span class="truncate">{{ p.name }}</span>
      </button>
      <div class="my-1 h-px bg-shell-deep" role="separator" />
      <button type="button" role="menuitem" :class="item" @click="run('add')">{{ t("profile.new") }}</button>
      <button type="button" role="menuitem" :class="item" @click="run('rename')">{{ t("profile.menuRename") }}</button>
      <button type="button" role="menuitem" :class="item" @click="run('import')">{{ t("profile.menuImport") }}</button>
      <button type="button" role="menuitem" :class="item" @click="run('export')">{{ t("profile.menuExport") }}</button>
      <button type="button" role="menuitem" :class="item" @click="run('reset')">{{ t("profile.menuReset") }}</button>
      <button type="button" role="menuitem" :class="item" :disabled="dev.profiles.length < 2" :title="dev.profiles.length < 2 ? t('errors.lastProfile') : undefined" @click="run('delete')">
        <span class="text-sensor">{{ t("profile.menuDelete") }}</span>
      </button>
    </div>
  </div>
</template>
