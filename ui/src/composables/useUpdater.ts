// In-app updates through tauri-plugin-updater: a quiet check 10 s after launch and every 6 h, never blocking the app.
// Outside Tauri it is a no-op, except ?mock=update, which simulates a release (&updateError=network|signature|package).
import { reactive } from "vue";
import { isTauri } from "../api/client";

export const RELEASES_URL = "https://github.com/lucasbrito-wdt/attack-shark-configurator/releases";

/** The webview does not open target=_blank links; the opener capability allows only this releases URL. */
export async function openReleases(): Promise<void> {
  if (!isTauri()) return void window.open(RELEASES_URL, "_blank", "noopener");
  try {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(RELEASES_URL);
  } catch {
    // nothing to do: the link text still names the page
  }
}

export type UpdatePhase = "idle" | "checking" | "upToDate" | "available" | "downloading" | "installing";
export type UpdateError = "" | "network" | "signature" | "package";

/** What the updater error text means for people. Linux packages it cannot replace go to the package manager. */
export function updateErrorKind(message: string, linux: boolean): Exclude<UpdateError, ""> {
  if (/signature|minisign|signed for/i.test(message)) return "signature";
  if (linux && /\.deb|install package|binary format|not found in the response|fallback platforms/i.test(message)) return "package";
  return "network";
}

interface Pending {
  version: string;
  date?: string;
  body?: string;
  download(onEvent: (e: { event: "Started"; data: { contentLength?: number } } | { event: "Progress"; data: { chunkLength: number } } | { event: "Finished" }) => void): Promise<void>;
  install(): Promise<void>;
}

const query = typeof location === "undefined" ? new URLSearchParams() : new URLSearchParams(location.search);
const mocked = query.get("mock") === "update";
// the mock stands in for a Linux .deb install when asked to fail like one
const linux = (typeof navigator !== "undefined" && /linux/i.test(navigator.userAgent)) || (mocked && query.get("updateError") === "package");

const state = reactive({
  phase: "idle" as UpdatePhase,
  error: "" as UpdateError,
  /** the error came from a check the person asked for (automatic checks stay silent) */
  manual: false,
  currentVersion: "",
  version: "",
  date: "",
  notes: "",
  total: 0,
  done: 0,
  open: false,
});
let pending: Pending | null = null;
let started = false;

/** A fake release for the browser: notes, then a download in chunks; failures on request. */
function mockCheck(): Pending {
  const fail = query.get("updateError");
  const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
  return {
    version: "0.2.0",
    date: "2026-11-19T12:00:00Z",
    body: "- Battery level over the Bluetooth battery service\n- Interface in Português, English and Español\n- DPI writing check for the X6",
    async download(onEvent) {
      const total = 8_400_000;
      onEvent({ event: "Started", data: { contentLength: total } });
      for (let sent = 0; sent < total; sent += 700_000) {
        await wait(120);
        if (fail && sent > total / 2) throw new Error(fail === "signature" ? "minisign: signature verification failed" : fail === "package" ? "Failed to install .deb package" : "error sending request");
        onEvent({ event: "Progress", data: { chunkLength: Math.min(700_000, total - sent) } });
      }
      onEvent({ event: "Finished" });
    },
    install: async () => void (await wait(600)),
  };
}

async function check(manual = false): Promise<void> {
  if (!(isTauri() || mocked) || state.phase === "checking" || state.phase === "downloading" || state.phase === "installing") return;
  Object.assign(state, { phase: "checking", error: "", manual });
  try {
    if (mocked) {
      state.currentVersion ||= "0.1.0";
      pending = mockCheck();
    } else {
      const [{ check: checkRemote }, { getVersion }] = await Promise.all([import("@tauri-apps/plugin-updater"), import("@tauri-apps/api/app")]);
      state.currentVersion ||= await getVersion();
      pending = await checkRemote();
    }
    if (!pending) return void (state.phase = "upToDate");
    Object.assign(state, { phase: "available", version: pending.version, date: pending.date ?? "", notes: pending.body ?? "" });
  } catch (e) {
    Object.assign(state, { phase: pending ? "available" : "idle", error: updateErrorKind(String(e), linux) });
  }
}

/** Download with progress, install, relaunch. On failure the app keeps running and the offer stays. */
async function install(): Promise<void> {
  if (!pending || state.phase === "downloading" || state.phase === "installing") return;
  Object.assign(state, { phase: "downloading", error: "", manual: true, total: 0, done: 0 });
  try {
    await pending.download((e) => {
      if (e.event === "Started") state.total = e.data.contentLength ?? 0;
      else if (e.event === "Progress") state.done += e.data.chunkLength;
    });
    state.phase = "installing";
    await pending.install();
    if (mocked) return void Object.assign(state, { phase: "upToDate", open: false, currentVersion: state.version });
    const { relaunch } = await import("@tauri-apps/plugin-process");
    await relaunch();
  } catch (e) {
    Object.assign(state, { phase: "available", error: updateErrorKind(String(e), linux) });
  }
}

/** Checks 10 s after launch, then every 6 h. */
function start(): void {
  if (started || !(isTauri() || mocked)) return;
  started = true;
  if (mocked) state.currentVersion = "0.1.0";
  else void import("@tauri-apps/api/app").then(({ getVersion }) => getVersion()).then((v) => (state.currentVersion = v)).catch(() => {});
  setTimeout(() => void check(), mocked ? 800 : 10_000);
  setInterval(() => void check(), 6 * 60 * 60 * 1000);
}

export const updatesSupported = (): boolean => isTauri() || mocked;

export function useUpdater() {
  return { state, check, install, start };
}
