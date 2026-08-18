# drm-custom-colorfix (v2.2.0)

A minimal, fast, and standalone DRM color temperature and display calibration utility for Linux Wayland compositors (specifically **COSMIC Desktop Environment** / `cosmic-comp`), designed to correct screen color balance, fix warm/yellowish tints on replacement laptop panels, and maintain a constant, calibrated display white point.

> **Attribution & Origin:** This project is a streamlined, optimized fork of [jjo/drm-colortemp](https://github.com/jjo/drm-colortemp). It has been rewritten and focused into a single pure-Rust binary, stripping out legacy C code, day/night shifting schedules, and external applet dependencies in favor of **seamless background auto-activation** and **native Fedora RPM packaging**.

---

## The Problem

1. **Missing Wayland Gamma Protocols in COSMIC**:
   COSMIC DE does not yet implement native color management or the `wlr-gamma-control-unstable-v1` protocol ([cosmic-comp #2059](https://github.com/pop-os/cosmic-comp/issues/2059)), which prevents traditional utilities like `gammastep`, `wlsunset`, or Redshift from controlling screen color temperature.
2. **Off-White / Yellowish Replacement Displays**:
   Replacement laptop panels (e.g. BOE 144Hz panels) often have a native warm/yellowish color cast (~5800K–6200K) rather than a neutral 6500K or cooler white point, making white backgrounds look uncalibrated.

---

## How It Works

`drm-custom-colorfix` communicates directly with the Linux kernel's Direct Rendering Manager (DRM) subsystem to program hardware Look-Up Tables (LUTs) onto the GPU's CRTC display controllers:

1. **Automatic Boot & Session Calibration (`AUTO_ACTIVATE=1`)**:
   - The systemd service runs at early boot and stays active in the background.
   - When COSMIC starts and holds the DRM master lock on the active virtual terminal (VT 1), the daemon automatically executes a fast **0.05s background micro-bounce** (`VT 1 -> VT 2 -> VT 1`) to flash the calibrated gamma table directly into the GPU hardware without requiring any manual keypresses.
2. **Multi-GPU Auto-Detection**:
   - Automatically detects and applies color calibration to all active displays across integrated GPUs (e.g. Intel Iris Xe) and dedicated GPUs (e.g. NVIDIA RTX).
3. **Live Config Reloading**:
   - Watches `/etc/default/drm-custom-colorfix.conf` using Linux `inotify`. Any change to the file takes effect instantly.

---

## What's New in v2.2.0

- **Single Constant Color Mode**: Removed day/night sunset/sunrise schedule logic to provide 100% constant, reliable screen calibration.
- **Zero-Touch Background Auto-Activation**: Implemented `ioctl(VT_ACTIVATE)` / `ioctl(VT_WAITACTIVE)` to bypass Wayland compositor locks without manual TTY switches.
- **Zero Extra Dependencies**: Removed `chrono` and external time libraries for maximum performance and minimal binary footprint.
- **Native Fedora Packaging**: Full support for building and installing `.rpm` packages via `make rpm`.

---

## Configuration (`/etc/default/drm-custom-colorfix.conf`)

The configuration file is clean, minimal, and self-documenting:

```ini
# /etc/default/drm-custom-colorfix.conf
# Configuration for DRM Custom Colorfix

# Target color temperature in Kelvin (1000-10000). Default: 8000
TEMPERATURE=8000

# Brightness multiplier (0.1 - 1.0). Default: 1.0
BRIGHTNESS=1.0

# Automatic VT micro-switch on boot/session start (1 = enabled, 0 = disabled)
AUTO_ACTIVATE=1

# DRM device to calibrate (leave commented out to auto-detect all connected GPUs)
# DEVICE="/dev/dri/card1"
```

### Configuration Options Reference

| Key | Default | Description |
| :--- | :--- | :--- |
| `TEMPERATURE` | `8000` | Target color temperature in Kelvin (1000 – 10000). 8000K–8200K is ideal for correcting warm/yellow replacement displays. |
| `BRIGHTNESS` | `1.0` | Brightness multiplier from `0.1` to `1.0`. |
| `AUTO_ACTIVATE` | `1` | Automatically switches VT for 0.05s on startup/login to bypass compositor locks. |
| `DEVICE` | *(auto)* | Path to DRM card (e.g. `/dev/dri/card1`). Omit to auto-detect all GPUs. |
| `CONNECTOR` | *(empty)* | Optional connector filter (e.g. `eDP-1`, `HDMI-A-1`). |
| `GAMMA_SIZE` | `0` | Override hardware gamma LUT size (`0` = auto-detect). |
| `VERBOSE` | `0` | Enable verbose logging (`1` = enabled, `0` = disabled). |

---

## Installation & Quick Start (Fedora / RHEL)

### 1. Build and Install the RPM

```bash
cd /home/fus0g/Projects/cosmic-fix/drm-colortemp-custom

# Build the RPM package
make rpm

# Install or upgrade
sudo dnf install ./build-rpm/RPMS/x86_64/drm-custom-colorfix-2.2.0-1.fc44.x86_64.rpm
```

### 2. Install Configuration & Enable Service

```bash
# Copy configuration to /etc/default/
sudo cp ./drm-custom-colorfix.conf /etc/default/drm-custom-colorfix.conf

# Enable and start systemd service
sudo systemctl enable --now drm-custom-colorfix
```

### 3. Check Status and Logs

```bash
# Check service status
sudo systemctl status drm-custom-colorfix

# View real-time logs
journalctl -u drm-custom-colorfix -f
```

---

## CLI Usage

You can also run `drm-custom-colorfix` manually from your terminal:

```bash
# List all detected DRM devices, CRTCs, and connectors
drm-custom-colorfix -l

# Set temperature to 8000K
sudo drm-custom-colorfix -t 8000

# Set temperature with 80% brightness
sudo drm-custom-colorfix -t 8000 -b 0.8

# Specify an explicit DRM card (e.g. Intel or NVIDIA)
sudo drm-custom-colorfix -d /dev/dri/card1 -t 8000

# Apply temperature once from config and exit
sudo drm-custom-colorfix --apply -c /etc/default/drm-custom-colorfix.conf

# Reset display to standard 6500K neutral white
sudo drm-custom-colorfix -r
```

---

## Building from Source

Build requirements: Rust toolchain (`cargo`), `make`, `gcc`, `rpm-build`, `systemd-rpm-macros`.

```bash
# Build release binary
make build

# Run test suite
make test

# Build RPM package
make rpm

# Clean build artifacts
make clean
```

---

## Troubleshooting

### Color temperature doesn't apply on startup
- Make sure `AUTO_ACTIVATE=1` is set in `/etc/default/drm-custom-colorfix.conf`.
- Ensure the systemd service is active: `sudo systemctl status drm-custom-colorfix`.
- Check logs: `journalctl -u drm-custom-colorfix -n 30 --no-pager`.

### DNF says "Package is already installed"
- If upgrading with the same version number, run:
  ```bash
  sudo dnf reinstall ./build-rpm/RPMS/x86_64/drm-custom-colorfix-2.2.0-1.fc44.x86_64.rpm
  ```

---

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).

---

## Credits & Acknowledgments

- Upstream project: [jjo/drm-colortemp](https://github.com/jjo/drm-colortemp).
- Color temperature algorithm based on Tanner Helland's blackbody conversion formulas.
