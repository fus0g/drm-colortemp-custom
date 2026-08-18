# drm-custom-colorfix

A minimal, fast, and standalone DRM color temperature and display calibration utility for Linux Wayland compositors (specifically **COSMIC Desktop Environment** / `cosmic-comp`), designed to correct screen color balance, fix warm/yellowish tints on replacement laptop panels, and control display color temperature.

> **Attribution:** This project is a streamlined fork of the original [jjo/drm-colortemp](https://github.com/jjo/drm-colortemp). It has been stripped of unnecessary extras (legacy C codebase, desktop notifications, separate applets) to provide a focused, single-binary Rust tool, enhanced with **early-boot automatic calibration** and **Fedora RPM packaging**.

---

## The Problem

COSMIC Desktop Environment does not yet implement native color management or the `wlr-gamma-control-unstable-v1` Wayland protocol ([cosmic-comp #2059](https://github.com/pop-os/cosmic-comp/issues/2059)), which prevents traditional tools like `gammastep`, `wlsunset`, or Redshift from adjusting display colors.

Furthermore, replacement laptop screens often come with an overly warm, yellowish, or off-white native white point (~5800K–6200K) that requires software-level color correction (e.g. setting target temperature to 7500K–8200K).

---

## The Solution: How `drm-custom-colorfix` Works

`drm-custom-colorfix` manipulates the display hardware's gamma Look-Up Tables (LUTs) directly at the Linux kernel DRM (Direct Rendering Manager) layer:

1. **Automatic Early-Boot Application (Zero Keypresses Needed)**:
   - A systemd service runs `drm-custom-colorfix --apply` *before* the display manager (`cosmic-greeter`) and compositor (`cosmic-comp`) start.
   - The hardware gamma table is programmed into the GPU (Intel Iris Xe / NVIDIA) before any compositor locks it.
   - When COSMIC starts, it renders over the hardware-calibrated display directly.

2. **On-Demand TTY Switching (Mid-Session Adjustments)**:
   - When running inside an active COSMIC session, the background daemon monitors Virtual Terminal (VT) switches.
   - Pressing **`Ctrl` + `Alt` + `F3`** (TTY3) causes COSMIC to temporarily release the DRM hardware lock.
   - The daemon immediately writes the new gamma curves and returns seamlessly to COSMIC (**`Ctrl` + `Alt` + `F2`**).

---

## Features

- **Lightweight & Pure Rust**: Uses direct DRM ioctls via libc/FFI with zero complex dependencies.
- **Automatic Boot Calibration**: Applies your configured Kelvin color temperature before the graphical desktop launches.
- **Multi-GPU Support**: Works with integrated Intel GPUs (`/dev/dri/card1`), discrete NVIDIA GPUs (`/dev/dri/card0`), and external HDMI/DisplayPort monitors.
- **Config Live Reload**: Automatically reloads settings via inotify whenever `/etc/default/drm-custom-colorfix.conf` is edited.
- **Fedora RPM Packaging**: One-command RPM builds with `make rpm`.

---

## Quick Start (Fedora / RHEL / CentOS)

### 1. Build and Install the RPM Package

```bash
# Build the binary and RPM
make rpm

# Install the generated RPM
sudo dnf install ./build-rpm/RPMS/x86_64/drm-custom-colorfix-2.1.0-1.fc44.x86_64.rpm
```

### 2. Configure Your Target Color Temperature

Edit `/etc/default/drm-custom-colorfix.conf`:

```bash
sudo nano /etc/default/drm-custom-colorfix.conf
```

Set your preferred daytime and nighttime temperatures:
```ini
# Auto-detect all active GPUs and displays:
# (or specify DEVICE="/dev/dri/card1")

# Daytime temperature in Kelvin (default: 8200 to neutralize warm/yellowish screens)
DAY_TEMP=8200

# Nighttime temperature in Kelvin
NIGHT_TEMP=8200

# Schedule
SUNSET_HOUR=20
SUNRISE_HOUR=8
```

### 3. Enable and Start the Background Service

```bash
sudo systemctl enable --now drm-custom-colorfix
```

---

## CLI Usage

You can also run the tool manually from the command line or from a TTY:

```bash
# List all detected DRM devices, CRTCs, and connectors
drm-custom-colorfix -l

# Set temperature to 8200K on default device
sudo drm-custom-colorfix -t 8200

# Set temperature with brightness multiplier (80% brightness)
sudo drm-custom-colorfix -t 8200 -b 0.8

# Specify a particular GPU (e.g. NVIDIA or Intel)
sudo drm-custom-colorfix -d /dev/dri/card1 -t 8200

# Apply scheduled temperature once from configuration and exit
sudo drm-custom-colorfix --apply -c /etc/default/drm-custom-colorfix.conf

# Reset to default 6500K neutral daylight
sudo drm-custom-colorfix -r
```

---

## TTY Shortcuts (For Mid-Session Adjustments)

| Shortcut | Description |
| :--- | :--- |
| **`Ctrl` + `Alt` + `F3` $\rightarrow$ `F2`** | Apply scheduled temperature (`DAY_TEMP` or `NIGHT_TEMP`) |
| **`Ctrl` + `Alt` + `F4` $\rightarrow$ `F2`** | Force night temperature (`NIGHT_TEMP`) |
| **`Ctrl` + `Alt` + `F5` $\rightarrow$ `F2`** | Force day temperature (`DAY_TEMP`) |

---

## Build from Source

Requirements: `rust` / `cargo`, `gcc`, `make`, `systemd-rpm-macros`, `rpm-build`.

```bash
# Build release binary
make build

# Run test suite
make test

# Build RPM package
make rpm

# Install locally without RPM
sudo make install
```

---

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).

---

## Credits & Acknowledgments

- Forked from [jjo/drm-colortemp](https://github.com/jjo/drm-colortemp).
- Color temperature algorithm based on Tanner Helland's blackbody conversion formulas.
