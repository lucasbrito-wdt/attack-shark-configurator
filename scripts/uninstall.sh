#!/usr/bin/env bash
# Remove what scripts/install.sh installed.
set -euo pipefail

case "$(uname -s)" in
Darwin)
    for dir in /Applications "$HOME/Applications"; do
        for app in "$dir"/ATTACK\ SHARK*.app; do
            [ -d "$app" ] || continue
            echo "Removing $app"
            rm -rf "$app"
        done
    done
    echo "Local settings stay in ~/Library/Application Support (remove manually if wanted)."
    ;;
Linux)
    # names before the app was renamed (attack-shark-x3) are removed too
    rm -fv "$HOME/.local/bin/attack-shark-configurator.AppImage" "$HOME/.local/bin/attack-shark-x3.AppImage"
    for pkg in attack-shark-configurator attack-shark-x3; do
        command -v dpkg >/dev/null 2>&1 && dpkg -s "$pkg" >/dev/null 2>&1 || continue
        echo "Next, using sudo, this script will run: dpkg -r $pkg"
        read -r -p "Continue? [y/N] " answer
        if [[ "$answer" =~ ^[Yy]$ ]]; then
            sudo dpkg -r "$pkg"
        fi
    done
    for RULE in /etc/udev/rules.d/99-attack-shark.rules /etc/udev/rules.d/99-attack-shark-x3.rules; do
        [ -f "$RULE" ] || continue
        echo "Next, using sudo, this script will remove $RULE and reload udev rules."
        read -r -p "Continue? [y/N] " answer
        if [[ "$answer" =~ ^[Yy]$ ]]; then
            sudo rm -f "$RULE"
            sudo udevadm control --reload-rules
            sudo udevadm trigger
        fi
    done
    echo "Local settings stay in ~/.config/attack-shark-configurator (remove manually if wanted)."
    ;;
*)
    echo "Unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac
