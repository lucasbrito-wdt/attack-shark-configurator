# ATTACK SHARK Configurator

Unofficial open-source configurator for ATTACK SHARK mice (macOS and Linux). Tauri v2 shell over a Rust core, with a
Vue 3 web UI. The wire protocol is documented in `docs/PROTOCOL.md` (O = observed, I = inferred); the UI/backend
contract is in `docs/API.md`.

## Layout

- `crates/x3-core`: protocol, device layer (USB HID + BLE), profile model, storage, per-model capabilities. No UI.
- `src-tauri`: Tauri app shell (commands, events, state machine, macOS vibrancy).
- `ui`: web UI (Vue 3 + Vite). `ui/src/api` holds the typed client with a mock backend for plain-browser design work.
- `packaging/linux`: udev rule, deb postinst. `design/`: design plan, tokens and the app icon source.

## Build

Requirements: Rust, `bun`, and `cargo install tauri-cli --version "^2" --locked`.

    cd ui && bun install && cd ..
    cargo tauri dev                              # runs the Vite dev server and the app
    cargo tauri build                            # bundles for the current OS
    cargo test --workspace
    cargo build -p x3-core --no-default-features # USB only

Linux build dependencies: `libudev-dev`, `libdbus-1-dev`, `pkg-config`, plus the Tauri prerequisites
(`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`).

## Install

`scripts/install.sh` builds with `cargo tauri build`. macOS: copies the `.app` to `/Applications` (or `~/Applications`) and
ad-hoc signs it. Linux: installs the `.deb` (which ships the udev rule and reloads udev) or, without dpkg, the AppImage plus
the rule. `scripts/uninstall.sh` reverses it. Local settings are kept.

The macOS `.dmg` is produced by `cargo tauri build` (all targets); it needs a GUI session.

### Permissions

- macOS: grant Input Monitoring (System Settings > Privacy & Security) so HID access works, and Bluetooth for BLE mode.
- Linux: the udev rule `packaging/linux/99-attack-shark-x3.rules` gives the logged-in user access to the hidraw nodes.
  Reload with `udevadm control --reload-rules && udevadm trigger`, then replug the device.

## Environment variables

- `X3_LOG_RAW=1`: log every raw report sent and received.
- `X3_LOG_EVENTS=1`: print every raw USB input report and BLE notification as hex to stderr, and emit the `device-event-raw` event.
- `X3_NO_BLE=1`: do not start the Bluetooth backend.
- `X3_ALLOW_UNVERIFIED=1`: allow writes not verified on hardware: DPI on the X3/X6, variant T DPI above 10000, and any
  write over the 4K receiver or the X3 PRO cable. Without it, apply sends everything else and reports the DPI group as
  skipped (`unverified_encoding`); 4K writes fail per packet.

## Hardware probe (read-only)

Plug the cable or dongle, then run:

    cargo run -p x3-core --bin x3-probe -- --read

It lists the matching HID interfaces (usage page, usage, interface), sends the 0xA0 read-back request, prints the firmware
version and the raw 0x04 reply decoded under both DPI interpretations (family A `u-1` and variant T). It never writes
configuration. `--listen <seconds>` prints input reports and BLE notifications (raw hex on stderr). Without flags it only
lists devices. Send the output when reporting a model, so the X3/X6 DPI encoding can be marked verified.

## Battery

The level comes only from the vendor battery event (`0x40xx`, C1 also `0x41xx`, over USB and BLE FEE4) and is `null` until
the first one arrives. `charging` follows the vendor charge state, or the wired PID on the X3/X6. Over BLE, the X6, X3, R1
and X11 report a bogus level, so it stays `null` with `reason: "not-reported-over-ble"` (see `docs/API.md`). The standard Battery Service (0x2A19) is logged under
`X3_LOG_EVENTS=1` as `bas_battery=<n>` for comparison and never reaches the UI: it reports 100% regardless of the real level.

BLE peripherals are selected by exposing FEE3 + FEE4 (no name filter; the device advertises as "X6 Mouse2").

## Known unknowns

See section 7 of `docs/PROTOCOL.md`. Fields marked `// inferred:` in the code need confirmation with a USB capture.
The full resync command (0x0C) only runs from `reset_profile` with `confirm: true`.

## Capturing USB traffic

1. Run the original Windows app in a VM with USB passthrough, or on a Windows host.
2. Capture with Wireshark + USBPcap (Windows) or `usbmon` (Linux), filtering on the mouse VID/PID from section 1 of the protocol doc.
3. Change one setting at a time and note the report ID and payload bytes of each OUT report.
4. Compare with `X3_LOG_RAW=1` output from this app and update `docs/PROTOCOL.md`.

## Releases and updates

The app checks `releases/latest/download/latest.json` on GitHub and verifies updates against the public key in `src-tauri/tauri.conf.json`.

One-time setup: create the GitHub repo `lucasbrito-wdt/attack-shark-x3`, then store the signing key (generated with `cargo tauri signer generate -w ~/.tauri/attack-shark-x3.key`, never commit it):

```sh
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/attack-shark-x3.key
```

Cutting a release:

```sh
scripts/release.sh 0.2.0   # bumps versions, runs cargo check, prints the git commands
# run the printed commit / tag / push commands
```

Pushing a `v*` tag triggers `.github/workflows/release.yml`, which builds macOS (arm64 + x86_64) and Linux bundles and creates a draft release with `latest.json`. Publish the draft to roll the update out.

Caveats:

- Linux: auto-update only works for the AppImage; `.deb` users update through the package.
- macOS: ad-hoc signed builds change identity on every build, so macOS asks for Input Monitoring / Bluetooth again after an update unless the app is signed with a stable Developer ID.

## Disclaimer

This project is not affiliated with, endorsed by, or sponsored by ATTACK SHARK or its manufacturer. "ATTACK SHARK"
and the model names are trademarks of their respective owners and are used only to identify compatible hardware.
No vendor software, images or logos are distributed in this repository. The protocol documentation was produced
for interoperability.
