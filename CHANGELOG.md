# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-11-19

First public release.

### Added

- Configuration of DPI stages (with a color per stage), polling rate and lift-off distance.
- Angle snap, ripple control and motion sync on the models that support them.
- Lighting modes and colors.
- Button remapping using the official driver's encoding, and macros. Buttons the app does not expose (DPI keys, wheel)
  keep their factory action.
- Battery level in the window and in the menu bar / system tray.
- Read-back of the mouse state and a DPI-encoding check.
- Closing the window keeps the app running in the tray.
- Automatic updates verified against a signed manifest.
- Interface in Portuguese (Brazil), English and Spanish.
- Connection over USB cable, 2.4G receiver and Bluetooth LE. Over Bluetooth the mouse does not acknowledge writes, so
  the profile is marked as unconfirmed.
- Support for BK, V4 and XVI platform models. Only the X6 is verified on hardware (USB and Bluetooth LE); the other
  models are untested.
- macOS (Apple Silicon and Intel `.dmg`), Linux (`.deb` with udev rule, AppImage) and Windows (`.msi`, setup `.exe`)
  builds. Windows support has not been tested on real hardware.
- `shark-probe`, a read-only hardware probe for reporting new models.

[Unreleased]: https://github.com/lucasbrito-wdt/attack-shark-configurator/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/lucasbrito-wdt/attack-shark-configurator/releases/tag/v0.1.0
