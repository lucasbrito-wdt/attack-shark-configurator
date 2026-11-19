#!/usr/bin/env bash
# Build and install ATTACK SHARK Configurator for the current user (macOS or Linux) via `cargo tauri build`.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
CARGO="$(command -v cargo || echo "$HOME/.cargo/bin/cargo")"

if ! "$CARGO" tauri --version >/dev/null 2>&1; then
    echo "tauri-cli is required. Install it with: cargo install tauri-cli --version \"^2\" --locked" >&2
    exit 1
fi
command -v bun >/dev/null 2>&1 || { echo "bun is required to build the UI (https://bun.sh)" >&2; exit 1; }
export PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
(cd ui && bun install --frozen-lockfile)

BUNDLE_DIR="target/release/bundle"

case "$(uname -s)" in
Darwin)
    echo "Building the macOS app bundle (cargo tauri build --bundles app)..."
    "$CARGO" tauri build --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'
    # the newest bundle wins: older ones (e.g. from before a rename) can stay in target/
    APP="$(ls -td "$BUNDLE_DIR"/macos/*.app 2>/dev/null | head -n 1)"
    [ -n "$APP" ] || { echo "No .app produced under $BUNDLE_DIR/macos" >&2; exit 1; }
    DEST="/Applications"
    [ -w "$DEST" ] || DEST="$HOME/Applications"
    mkdir -p "$DEST"
    rm -rf "$DEST/$(basename "$APP")"
    cp -R "$APP" "$DEST/"
    # ad-hoc signature keeps a stable identity for the macOS permission prompts (best effort)
    if ! codesign --force --deep --sign - "$DEST/$(basename "$APP")"; then
        echo "Warning: ad-hoc codesign failed; macOS may re-ask permissions after each update" >&2
    fi
    echo "Installed to $DEST/$(basename "$APP")"
    echo "On first use macOS asks for Input Monitoring and Bluetooth permission."
    ;;
Linux)
    echo "Building the Linux bundles (cargo tauri build --bundles deb,appimage)..."
    "$CARGO" tauri build --bundles deb,appimage --config '{"bundle":{"createUpdaterArtifacts":false}}'
    DEB="$(ls -td "$BUNDLE_DIR"/deb/*.deb 2>/dev/null | head -n 1)"
    if [ -n "$DEB" ] && command -v dpkg >/dev/null 2>&1; then
        echo "Next, using sudo, this script will run: dpkg -i $DEB"
        echo "The package installs the udev rule to /etc/udev/rules.d/ and reloads udev (TAG uaccess, MODE 0660)."
        read -r -p "Continue? [y/N] " answer
        if [[ "$answer" =~ ^[Yy]$ ]]; then
            sudo dpkg -i "$DEB"
            echo "Installed the .deb. Replug the mouse or dongle if it was already connected."
        else
            echo "Skipped the .deb install."
        fi
    else
        APPIMAGE="$(ls -td "$BUNDLE_DIR"/appimage/*.AppImage 2>/dev/null | head -n 1)"
        [ -n "$APPIMAGE" ] || { echo "No .deb or AppImage produced under $BUNDLE_DIR" >&2; exit 1; }
        BIN_DIR="$HOME/.local/bin"
        mkdir -p "$BIN_DIR"
        install -m 755 "$APPIMAGE" "$BIN_DIR/attack-shark-configurator.AppImage"
        echo "Installed $BIN_DIR/attack-shark-configurator.AppImage"
        echo
        echo "Next, using sudo, this script will:"
        echo "  - copy packaging/linux/99-attack-shark.rules to /etc/udev/rules.d/"
        echo "  - run: udevadm control --reload-rules && udevadm trigger"
        echo "The rule lets your user open the mouse hidraw devices (TAG uaccess, MODE 0660)."
        read -r -p "Continue? [y/N] " answer
        if [[ "$answer" =~ ^[Yy]$ ]]; then
            sudo install -m 644 packaging/linux/99-attack-shark.rules /etc/udev/rules.d/99-attack-shark.rules
            sudo udevadm control --reload-rules
            sudo udevadm trigger
            echo "udev rule installed. Replug the mouse or dongle if it was already connected."
        else
            echo "Skipped the udev rule. Without it the app may not be able to open the device."
        fi
    fi
    ;;
*)
    echo "Unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac
