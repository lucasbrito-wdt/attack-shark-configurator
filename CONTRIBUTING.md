# Contributing

Thanks for helping. The most valuable contributions are test reports from real hardware, because only the X6 has been
verified so far.

## Development setup

Requirements: Rust (stable), [Bun](https://bun.sh) and the Tauri CLI.

```sh
cargo install tauri-cli --version "^2" --locked
cd ui && bun install && cd ..
cargo tauri dev
```

On Linux install the build dependencies listed in the [README](README.md#building-from-source). Use
`PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1` with `bun install` if any dependency tries to download a browser.

The UI also runs in a plain browser against a mock backend (`ui/src/api`), which is enough for layout and copy work:
`cd ui && bun run dev`.

## Running checks

CI runs these on Linux, macOS and Windows. Run them before opening a pull request.

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cd ui && bun run build        # type-checks (vue-tsc) and builds the UI
```

The UI build must exist before the Rust crates compile (`ui/dist` is embedded by Tauri).

## Code conventions

- Code, identifiers, file names, commit messages and comments are in English. Only user-visible UI text is localized,
  in `ui/src/locales/` (pt-BR, en, es). Add every new string to all three.
- Keep files under 500 lines. Split by responsibility instead of growing a file.
- Reuse before adding: search for an existing helper or pattern first.
- Keep changes focused. Do not refactor unrelated code in the same pull request.
- Protocol facts in docs are tagged **O** (observed on the wire or in the vendor binary) or **I** (inferred). Mark code
  that depends on an inference with an `// inferred:` comment, and do not promote an I to an O without evidence
  (a capture or a read-back from real hardware).
- Writes whose encoding is not verified on hardware must stay behind the unverified gate (`SHARK_ALLOW_UNVERIFIED=1`).
- Do not add vendor software, firmware, images or logos to the repository.

## Reporting a new or untested model

Open a [model request](https://github.com/lucasbrito-wdt/attack-shark-configurator/issues/new?template=model_request.yml)
with the model name, the USB VID:PID and the probe output. The probe is read-only and never writes configuration:

```sh
cargo run -p shark-core --bin shark-probe -- --read
```

Plug the cable or the receiver first. Paste the full output. To report that an already listed model works (or does not),
open a bug report or model request with the same output and say which connections you tried (USB, 2.4G, BLE) and which
features you checked. `--listen <seconds>` additionally prints input reports and BLE notifications.

To capture traffic against the vendor app, see the protocol notes in [docs/PROTOCOL.md](docs/PROTOCOL.md).

## Pull requests

1. Open an issue first for anything larger than a small fix, so the approach can be agreed.
2. Fork, then branch from `main` with a descriptive name.
3. Make the change with tests where logic is involved (protocol encoders and parsers are unit tested).
4. Run the checks above.
5. Open the pull request and fill in the template. Describe what changed, why, and how you tested it, including the
   hardware used if the change touches the device layer.

Commit messages follow the conventional style already in the history (`feat(bk): ...`, `fix: ...`).

## License

This project is licensed under [GPL-3.0-only](LICENSE). By submitting a contribution you agree that it is licensed under
the same terms.

By participating you also agree to the [Code of Conduct](CODE_OF_CONDUCT.md).
