# CPU Tweaks

A native Linux GUI for CPU frequency and power management. Reads all available options from your hardware and lets you configure them with a single click.

![Rust](https://img.shields.io/badge/Rust-1.70%2B-orange)
![License](https://img.shields.io/badge/License-MIT-blue)
![Platform](https://img.shields.io/badge/Platform-Linux-green)

## Features

- **Auto-detects your CPU driver** — Intel P-State, AMD P-State, acpi-cpufreq, or generic cpufreq
- **Governor selection** — switch between `performance`, `powersave`, `ondemand`, `conservative`, `schedutil`, etc.
- **Energy Performance Preference (EPP)** — choose from `default`, `performance`, `balance_performance`, `balance_power`, `power` (Intel HWP / AMD P-State)
- **Frequency limits** — set min/max scaling frequency with sliders
- **Turbo Boost control** — enable/disable Intel Turbo Boost or AMD Boost
- **Intel P-State tuning** — min/max performance percentage, HWP dynamic boost
- **Live frequency monitor** — real-time per-core frequency bars with color coding
- **Current state display** — see all active settings at a glance
- **Privilege escalation** — uses `pkexec` to apply changes (no need to run as root)
- **Dark themed UI** — clean, modern look built with egui

## Supported Drivers

| Driver | Governor | EPP | Turbo | Perf % | Notes |
|--------|----------|-----|-------|--------|-------|
| `intel_pstate` | ✅ | ✅ | ✅ | ✅ | Full support including HWP |
| `amd-pstate` | ✅ | ✅ | ✅ | ❌ | EPP available on amd-pstate-epp |
| `acpi-cpufreq` | ✅ | ❌ | ✅ | ❌ | Classic frequency scaling |
| Generic | ✅ | ❌ | ❌ | ❌ | Basic governor + freq control |

## Screenshots

The GUI shows:
- **Left panel**: Live per-core frequency bars (green/orange/red based on load) + current state summary
- **Right panel**: Governor buttons, EPP selector, frequency sliders, turbo toggle, Intel P-State knobs, and Apply button

## Architecture

```
src/
├── main.rs              # Entry point, window setup
├── cpu/
│   ├── mod.rs
│   ├── sysfs.rs         # Read CPU info from /sys/devices/system/cpu/
│   └── apply.rs         # Build shell commands, run via pkexec
└── gui/
    ├── mod.rs
    ├── app.rs           # App state, refresh logic, apply logic
    ├── views.rs         # All UI drawing (live bars, controls, cards)
    └── theme.rs         # Dark theme colors and styling
```

## How It Works

1. **Read**: Scans `/sys/devices/system/cpu/` to detect driver, available governors, EPP options, frequency ranges, turbo support
2. **Display**: Shows all detected options in the GUI — only what your hardware actually supports
3. **Apply**: Builds a shell script with the changes and runs it via `pkexec bash -c '...'`
4. **Refresh**: Re-reads sysfs every 2 seconds to show live frequencies

## Build & Run

```bash
# Build
cargo build --release

# Run (no root needed — pkexec handles privilege escalation on apply)
./target/release/cpu-tweaks
```

## Install as .deb Package

```bash
# One-step build + install
./install.sh

# Or manually:
cargo install cargo-deb
cargo deb
sudo dpkg -i target/debian/cpu-tweaks_*.deb
```

After installing, find **CPU Tweaks** in your application menu under System/Settings.

## Dependencies

### Build
- Rust 1.70+
- `libgtk-3-dev`, `libglib2.0-dev`

### Runtime
- `libgtk-3-0`, `libglib2.0-0`
- `polkit-1` (for `pkexec`)
- Linux kernel with cpufreq support

## Equivalent CLI Commands

What this tool does behind the scenes (example for Intel P-State):

```bash
# Set governor to powersave on all cores
echo 'powersave' | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor

# Set max frequency to 2700 MHz
echo '2700000' | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_max_freq

# Enable turbo boost
echo '0' | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

# Set EPP to power saving
echo 'power' | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference

# Set performance percentage range
echo '10' | sudo tee /sys/devices/system/cpu/intel_pstate/min_perf_pct
echo '80' | sudo tee /sys/devices/system/cpu/intel_pstate/max_perf_pct
```

## License

MIT
