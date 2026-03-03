# SleepLight

SleepLight is a BLE-connected bedside light built around an ESP32-C3, with an iOS companion app for color and brightness control.

## Repository layout

- `firmware/` - Rust firmware for the ESP32-C3 (BLE peripheral + LED ring control).
- `ios/` - SwiftUI iOS app that scans, connects, and sends color/brightness updates over BLE.
- `hardware/` - KiCad project, manufacturing outputs, and mechanical assets (`.kicad_*`, Gerbers, STEP/STL).
- `silkscreen.ai` - Illustrator source for silkscreen/artwork.

## Quick start

### 1) Firmware (ESP32-C3)

Prereqs:
- Rust toolchain (stable) with `riscv32imc-unknown-none-elf`
- `probe-rs` installed
- ESP32-C3 board connected via debug probe

From `firmware/`:

```bash
cargo run
```

This uses the configured runner (`probe-rs run --chip esp32c3 ...`) to build and flash.

### 2) iOS app

Prereqs:
- Xcode (current version)
- iOS device or simulator with Bluetooth support (device recommended)

Open and run:
- `ios/SleepLight.xcodeproj`
- Select the `SleepLight` target and run.

The app scans for a BLE peripheral named `SleepLight` and controls RGB + brightness.

## Current BLE contract

- Peripheral name: `SleepLight`
- Light service UUID: `f3e0c001-8b6f-4d2e-a2d0-6b9c3f2a0000`
- State characteristic UUID: `f3e0c002-8b6f-4d2e-a2d0-6b9c3f2a0000`
- Payload format: 4 bytes `[R, G, B, brightness]`

## Licensing

See [licenses](./LICENSES/README.md) for scope mapping.
