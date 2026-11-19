# UI <-> backend contract

The web UI talks to the Rust shell (`src-tauri`) only through the typed client in `ui/src/api/client.ts`.
Types live in `ui/src/api/types.ts` and mirror `src-tauri/src/dto.rs` (+ `crates/shark-core/src/model.rs`).
Rust tests assert the JSON key names of every DTO, so renaming a field on one side breaks the build.
Everything is camelCase. Outside Tauri (plain browser) the client uses an in-memory mock with the same
contract; `?mock=offline|asleep|wired|ble|ble-nobas|fail|retry|verify-ok|verify-inconclusive` in the URL, or `window.__sharkMock` at runtime
(`setMode(mode, awake)`, `behavior = "ok" | "fail" | "retry-then-ok"`), selects scenarios. `?model=<id>` picks one of the
models in `ui/src/api/mockModels.ts` (generated from the Rust registry): x6, f1, x8-pro, r1, x11, g3, v3, v6, x1, v5-se,
r6 (XVI), x3-pro-v4 (V4).

## Conventions

- Every command is async and rejects with `ApiError { code, message }`. Codes: `invalid_input`,
  `not_found`, `confirm_required`, `last_profile`, `busy`, `io`, `unverified_encoding`, `unsupported_model`, and for
  `read_back` also `unsupported`, `timeout`, `decode_failed`.
  `message` is English, for logs, not for display.
- Unverified DPI is a partial success, not an error: when the DPI packet uses an encoding not verified on hardware and the
  app was not started with `SHARK_ALLOW_UNVERIFIED=1`, apply leaves out only that packet and sends every other group.
  `ApplyResult.skippedGroups` and `ApplyProgress.skippedGroups` then hold `{ group: "dpi", code: "unverified_encoding" }`.
  It happens when `capabilities.dpiVerified === false` (today X3/X6 and V4), or `dpiEncoding === "t"` with any stage above
  10000 DPI (the multiplier bits are copied from the vendor, not understood). The first full apply after a link comes up
  follows the same rule. The skipped DPI is NOT counted as applied: `Snapshot.pendingGroups` keeps
  `{ group: "dpi", code: "unverified_encoding" }` and `pendingChanges` stays true until a write of 0x04 succeeds;
  every later apply re-includes it (sent once the encoding is verified or the variable is set, skipped and reported again
  otherwise). The app verifies the encoding itself with `verify_dpi_encoding` (below); `SHARK_ALLOW_UNVERIFIED=1` is a
  developer override only.
  The `unverified_encoding` error code is thrown only by a codec that cannot single out its DPI packet.
- Writes over the 4K receiver (FA65) and the X3 PRO cable are also unverified: without the variable they fail per packet
  and show up as `apply-progress` `failed`. XVI writes wait for the device ack; a missing ack is `failed`, never `ok`.
- `unsupported_model`: no codec for the model's platform, or the connected model is not identified yet (a shared BK
  receiver before its first report, a V4 shared PID until command 0x8F answers). Nothing is sent; the model is never guessed.
- Profile and macro ids are list positions. Delete invalidates ids: always use the list from the last response.
- Colors are `"#RRGGBB"`. DPI is the real value: `dpiMin..dpiMax` in steps of `dpiStep` up to `dpiHighStepFrom`, then in
  steps of 100 (when `dpiHighStepFrom` is not null); `dpi: 0` is an unused stage slot.
  `activeStage` and `dpi-stage-changed.stage` are 0-based. `polling` is Hz, `lod` is mm.
- `debounce` is the raw value; milliseconds = `debounce * capabilities.debounceMsPerUnit` (BK 2, inferred from the
  vendor label; V4 1).
- Invalid patches are rejected as a whole; nothing is persisted or sent.
- Settings are persisted on every mutating command. Device writes happen right after, asynchronously.

## Commands

| Command (client method) | Args | Result | Notes |
|---|---|---|---|
| `get_snapshot` (`getSnapshot`) | | `Snapshot` | status, selected profile, profile list, macros, capabilities, `pendingChanges`, `pendingGroups`, `dpiEncoding` |
| `list_profiles` | | `ProfileSummary[]` | |
| `create_profile` | `name` | `Snapshot` | new default profile, selected |
| `rename_profile` | `id, name` | `Snapshot` | name 1..48 chars |
| `delete_profile` | `id` | `Snapshot` | `last_profile` if it is the only one |
| `select_profile` | `id` | `Snapshot` | does not touch the device; `pendingChanges` becomes true, call `apply_all` |
| `import_profile` | `path` | `Snapshot` | sanitized, appended, selected. Use `pickOpenPath()` |
| `export_profile` | `id, path` | `null` | use `pickSavePath(name)` |
| `reset_profile` | `id, confirm` | `SettingsResult` | `confirm` must be `true` (`confirm_required` otherwise). Resets to defaults keeping the name. For the selected profile it sends the risky 0x0C resync plus every group. Ask the user first. `busy` while an apply is in flight |
| `update_settings` | `patch: SettingsPatch` | `SettingsResult` | partial patch, persists, sends only the changed groups in spec 3.8 order (0x04, 0x05, 0x06, 0x08, 0x09) |
| `set_button_action` | `button, action: ActionRef` | `SettingsResult` | macro binding copies the library macro at bind time (later `update_macro` does not rebind) |
| `list_actions` | | `ActionCatalog` | preset actions by category, HID key list, modifier bits |
| `list_macros` / `create_macro` / `update_macro` / `delete_macro` | `input: MacroInput`, `id` | `Macro[]` | fresh library. Events: key down/up plus `delayMs`; `loopMode` count / untilKey / whileHeld; at most `capabilities.maxMacroEvents` events |
| `apply_all` | | `ApplyResult` | rewrites every group of the selected profile (no 0x0C) |
| `read_back` (`readBack`) | | `ReadBack` | reads the device's current settings (0xA0 read-back, family-b 3.6). Read-only: never writes configuration and changes no profile. See below |
| `verify_dpi_encoding` (`verifyDpiEncoding`) | | `DpiVerification` | read-only check of which DPI encoding the device stores; the verdict is persisted and, when it matches the model's encoding, enables DPI writes. See below |

`read_back`: only when `capabilities.supportsReadBack` and a cable or 2.4G receiver link is up (not Bluetooth, not the
4K receiver / X3 PRO cable). `ReadBack` is the `Profile` shape without `id`/`name`, plus `firmware`, with every field
nullable: `null` means the reply does not carry it or it is ambiguous (e.g. two slider values that encode to the same
bytes, brightness in a mode that does not send it, a flag the model's 0x04 layout lacks). `buttons` is always `null`
(0x08 not decoded). Values are decoded by re-encoding with the same builders apply uses, so they are only as verified as
the write path. Errors: `unsupported` (model or link), `unsupported_model` (not identified yet), `busy` (an apply is in
flight), `timeout` (the device never reported the data ready), `decode_failed` (wrong reply header, or family B firmware
other than 0x10xx, which answers the undecoded 0x0A bulk read), `io`. While it runs (up to ~5 s) applies are held and
sent when it ends.

`verify_dpi_encoding` returns `{ status, format?, reason? }`, `status` one of:
- `verified`: `format` (a `capabilities.dpiEncoding` name) is the only encoding the stored block fits. If it equals
  `capabilities.dpiEncoding`, `capabilities.dpiVerified` becomes true and DPI writes are sent. If not, `reason` says DPI
  writes stay disabled. A model that is verified in the table answers `verified` at once, without reading.
- `inconclusive`: `reason` says why, e.g. every stored stage fits both encodings, the reply checksum is wrong, the
  firmware answers the undecoded 0x0A bulk read, or the platform has no check (XVI, V4).
- `unsupported_link`: no cable or 2.4G receiver is up. Bluetooth is not enough: the vendor's BLE read-back blob
  (family-b 4.3) is not decoded, and `X3.exe` never reads over BLE. The UI should ask for the cable or the receiver.
- `timeout`: the device never reported the data ready.
Errors (rejections): `busy` (an apply or a read is running) and `unsupported_model` (not identified yet).

The method (family A X3/X6, family-b unknown #1): the check reads firmware 0x0B and the stored 0x04 block (`A0 04 38`
plus GetFeature, as `read_back`), and nothing else. Only the DPI fields count. Two writers target this PID and code.
`u-1` (`X3.exe`) stores `units - 1` as lo/hi bytes. The R1 table stores a sensor code, a 0/1 half-step flag, and `[6]`
bit i only when stage i is above 220 units. A table block is refuted when a stage has a code, flag and `[6]` bit that no
table row produces. The X6 factory 26000 DPI stage (hi byte 2) and 400 DPI (code 7) are examples. A `u-1` block is
refuted only by a stage above the model's maximum. Every table block fits `u-1` on these models, so the table can never
be confirmed. Blocks with every stage below about 10000 DPI usually fit both and stay `inconclusive`.

Automatic run: once per cable or receiver connection, after the model is identified. It runs only when
`capabilities.dpiVerified` is false and no verdict is stored. Judged verdicts (`verified` or `inconclusive`) are
persisted per model with the firmware version and are exposed as `Snapshot.dpiEncoding`
(`{ verified, format, at, firmware }` or `null`). Link errors and timeouts are not stored. A stored verdict suppresses
the automatic run; call the command again to re-check. While the check runs, applies are held and sent when it ends.
When it ends, call `get_snapshot` to pick up `capabilities.dpiVerified`.

`ApplyResult.status`: `queued` (packets handed to the writer, outcome arrives as `apply-progress`; `packets: 0`
means another apply is running and the changes will be flushed when it finishes), `nothing`, `no-device`, `asleep`.
The settings are saved in all four cases.

`ActionRef`: `{kind:"preset", id}` | `{kind:"shortcut", key, mods}` | `{kind:"macro", macroId}`.
Output buttons always carry `label`, the text to display.

## Events

| Event | Payload | When |
|---|---|---|
| `device-status` | `DeviceStatus` | on every visible change (link up/down, heartbeat, sleep/wake via heartbeat watchdog, name) |
| `dpi-stage-changed` | `{ stage, dpi }` | the mouse button changed the DPI stage. The stored profile's `activeStage` is already updated |
| `apply-progress` | `ApplyProgress` | `pending` when packets are queued, then `ok` or `failed`. `retry` (with `attempt`) is part of the contract but the Rust sender does not report retries yet; only the mock emits it `unconfirmed: true` on an `ok` sent over Bluetooth: the mouse acknowledges no BLE write, so the profile stays pending |
| `device-event-raw` | `{ event }` | only with `SHARK_LOG_EVENTS=1`; Debug text of the parsed event (not raw bytes; raw hex goes to stderr) |
| `dpi-verification` | `{ state: "running" }` or `{ state: "done", result: DpiVerification }` | a DPI encoding check started (manual or automatic) / ended. Immediate answers (`unsupported_link`, verified in the table) emit only `done` |

Subscribe with `on("device-status", handler)`; it returns an unsubscribe function.

## Device status honesty

- `battery.level` is `null` until a vendor battery event (type `0x40xx`/`0x41xx`) or, over BLE, a BAS value arrives. Scale per model: tenths
  (level 1..10 -> 10..100 %; codes 0x10/0x85/0x4D) or percent (0..100). Out-of-range levels keep the previous value.
  It resets to `null` when the last link drops.
- Over BLE, models whose code is 0x85, 0x4D, 0x10 or 0x55 (X6, X3, R1, X11; `capabilities.batteryOverBle === false`) send a
  bogus vendor value, which is discarded: the level falls back to the BLE Battery Service (see `battery.source`).
- `battery.charging` is `boolean | null`: family B uses the vendor state (3 = charging, 2 = full, other states unknown),
  C1 uses state 2 or 3; family A (X3/X6) has no charge state, so it is true while the wired PID is present.
  `null` when disconnected or unknown.
- `battery.lastKnown` is `{ level, charging, link: "usb" | "dongle" | "ble", at }` (`at` RFC 3339 UTC) or `null`. It is
  the last accepted battery reading (vendor or BLE BAS) of the current model, persisted in the settings file, and is NOT cleared when
  the link drops or when BLE discards the level. Show it when `level` is `null` (e.g. the X6 over BLE), with its age.
- `battery.source` is `"vendor-event"` when the level comes from the vendor battery event, `"ble-bas"` when it comes from the
  standard BLE Battery Service (0x2A19, read on connect and on every notification), else `"none"`. The vendor event wins
  when both exist; BAS is used only on a BLE link. A BAS byte of 0..=100 is the level; 101..=255 is not decoded (logged,
  the previous level stays). BAS has no charge state, so `charging` stays `null` with `"ble-bas"` on models whose vendor
  battery is discarded over BLE. A BAS reading also updates `lastKnown` with `link: "ble"` (unless a vendor level is in use).
- `battery.reason` is `"not-reported-over-ble"` only when the link is BLE, the vendor battery is discarded there and no
  BAS value is known; otherwise `null`.
- `mode` is the highest-priority link present (wired > dongle > ble). `model` is the USB product string or BLE name
  (e.g. "X6 Mouse2"), `null` when unknown.
- `awake` is false when no device is connected, when a dongle link has no heartbeat for 6 s, or when the heartbeat says asleep.
  While asleep, applies return `asleep`.
- Event types are `category | modelCode` (e.g. BLE `10 10 02 00` = DPI stage 2 on a model with code 0x10).
  Unknown categories are logged under `SHARK_LOG_EVENTS=1` and otherwise ignored.

## Capabilities

Built from the detected model's `ModelSpec` (`crates/shark-core/src/platform`). With no device, the last detected model
(persisted) is used, else the X6. `modelId` / `modelName` / `platform` (`"bk" | "xvi" | "v4"`) / `modelCode` identify it;
when a device is detected as another model, call `get_snapshot` again (watch `device-status.model`).
`dpiEncoding` is `"a" | "t" | "h" | "direct"`, `dpiVerified` false means applies skip the DPI group (`skippedGroups`).
`dpiVerified` is true when the model table says so, or when a stored `verify_dpi_encoding` verdict confirms `dpiEncoding`.
`pollingRates` lists 125..8000 Hz as the model supports on the current link (XVI wired is capped at 1000 Hz); call
`get_snapshot` again when `device-status.mode` changes. Ranges (`debounce`, `sleepMinutes`, `brightness`...) are
`{min, max}`. `supportsAngleSnap` / `supportsRippleControl` / `supportsMotionSync` and the `debounce`, `sleepMinutes`,
`deepSleepMinutes` ranges come from what the model's codec writes (a test checks every model): a `false` flag or a `null`
range means the profile field is `null` and `update_settings` rejects it with `invalid_input`. Today: BK variant T R1
pages lack motion sync, V3 pages lack ripple, C1 X11 PRO V2 lacks motion sync; XVI has no debounce or deep sleep and a
1..30 min sleep (15..1800 s on the wire); V4 CH585 has the three flags and debounce (range inferred), no sleep; V4
Pan1080 has none. Each `lightingModes` entry has `usesBrightness` (the codec sends the brightness only in modes 1/9 on
family A, 1/5 on family B, every mode on C1); `usesColor` and `usesSpeed` are `null`: the packet always carries them and
which modes use them is not decoded. `buttons` are the slots the vendor UI exposes; the profile holds 18. `supportsMacros` is false on the
F1 / X11 PRO V2 and on every page from the X11 PRO (MS7) on, whose 111-byte macro format is not decoded. `lightingModes` is empty when the model
has no lighting. `supportsLighting` mirrors whether the vendor shows the lighting section. `batteryOverBle` and
`supportsReadBack` describe the battery rule above and the 0xA0 read-back.
