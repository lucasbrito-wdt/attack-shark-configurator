<script setup lang="ts">
// UI language. Native select: keyboard and screen readers get the platform behavior for free.
import { useI18n } from "vue-i18n";
import { LOCALES, setLocale, type Locale } from "../../i18n";
import AppIcon from "../ui/AppIcon.vue";

const { t, locale } = useI18n();
</script>

<template>
  <label class="relative flex h-8 items-center gap-2 rounded-key px-3 text-micro text-wheel-muted transition-colors duration-150 ease-actuation hover:bg-shell/80 has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-led-deep">
    <AppIcon name="globe" class="size-4" />
    <span class="sr-only">{{ t("common.language") }}</span>
    <select
      :value="locale"
      class="min-w-0 flex-1 cursor-pointer appearance-none bg-transparent pr-5 font-medium text-wheel outline-none"
      @change="setLocale(($event.target as HTMLSelectElement).value as Locale)"
    >
      <option v-for="l in LOCALES" :key="l.id" :value="l.id" :lang="l.id">{{ l.name }}</option>
    </select>
    <AppIcon name="chevron" class="pointer-events-none absolute right-3 size-3.5" />
  </label>
</template>
