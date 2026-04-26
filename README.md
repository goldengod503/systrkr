# systrkr

Lightweight native [COSMIC](https://system76.com/cosmic) panel applet that shows live CPU and GPU usage as filled-area sparklines, plus a popup with system details.

![systrkr popup with CPU and GPU details](assets/systrkr_view.png)

## Features

- Two live sparklines in the panel (CPU + GPU) at 2 Hz, 30-second history
- Auto-detects NVIDIA (NVML), AMD (sysfs), and Intel (sysfs) GPUs
- Popup with CPU/GPU model, temperatures, RAM/swap usage, load average, and VRAM
- "Open System Monitor" button that launches `cosmic-monitor` or falls back to `gnome-system-monitor`
- Native COSMIC theming — follows your accent color and light/dark mode
- Zero configuration

## Requirements

- Rust 1.78+ (2024 edition)
- COSMIC desktop
- For NVIDIA GPU support: NVIDIA driver with NVML library installed

## Install

```bash
just install
```

Then add the applet via `cosmic-settings → Panel → Configure panel applets`.

To install without NVIDIA support (e.g., on AMD-only systems):

```bash
cargo build --release --no-default-features
just install
```

## Uninstall

```bash
just uninstall
```

## Development

```bash
just check    # cargo check
just test     # cargo test
just clippy   # lints
just run      # run the applet standalone for sanity-checking
```

## License

MIT
