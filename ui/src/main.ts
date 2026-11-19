import { createPinia } from "pinia";
import { createApp } from "vue";
import { isTauri } from "./api/client";
import App from "./App.vue";
import { currentLocale, i18n, setLocale } from "./i18n";
import "./style.css";

document.documentElement.dataset.shell = isTauri() ? "native" : "web";
void setLocale(currentLocale(), false);
createApp(App).use(createPinia()).use(i18n).mount("#app");
