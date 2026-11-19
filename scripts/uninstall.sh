#!/usr/bin/env bash
# Remove what scripts/install.sh installed.
set -euo pipefail

case "$(uname -s)" in
Darwin)
    for dir in /Applications "$HOME/Applications"; do
        for app in "$dir"/*X3*.app; do
            [ -d "$app" ] || continue
            echo "Removing $app"
            rm -rf "$app"
        done
    done
    echo "Local settings stay in ~/Library/Application Support (remove manually if wanted)."
    ;;
Linux)
    rm -fv "$HOME/.local/bin/attack-shark-x3.AppImage"
    if command -v dpkg >/dev/null 2>&1 && dpkg -s attack-shark-x3 >/dev/null 2>&1; then
        echo "Next, using sudo, this script will run: dpkg -r attack-shark-x3"
        read -r -p "Continue? [y/N] " answer
        if [[ "$answer" =~ ^[Yy]$ ]]; then
            sudo dpkg -r attack-shark-x3
        fi
    fi
    RULE=/etc/udev/rules.d/99-attack-shark-x3.rules
    if [ -f "$RULE" ]; then
        echo "Next, using sudo, this script will remove $RULE and reload udev rules."
        read -r -p "Continue? [y/N] " answer
        if [[ "$answer" =~ ^[Yy]$ ]]; then
            sudo rm -f "$RULE"
            sudo udevadm control --reload-rules
            sudo udevadm trigger
        fi
    fi
    echo "Local settings stay in ~/.config/attack-shark-x3 (remove manually if wanted)."
    ;;
*)
    echo "Unsupported OS: $(uname -s)" >&2
    exit 1
    ;;
esac
