# drm-custom-colorfix (v2.3.0)

A minimal, fast, and standalone display color temperature and calibration utility for Linux desktop environments (**GNOME / Wayland** and **COSMIC Desktop / DRM**). Designed to correct screen color balance, fix warm/yellowish tints on replacement laptop panels, and maintain a constant, calibrated display white point.

> **Attribution & Origin:** This project is a streamlined, optimized fork of [jjo/drm-colortemp](https://github.com/jjo/drm-colortemp). It has been rewritten in pure Rust and extended with dual-backend support: native **GNOME/Wayland** color management integration (`colord` + ICC VCGT profiles) alongside the zero-touch **DRM/COSMIC** auto-activation engine and **native Fedora RPM packaging**.

---

## Dual Backend Architecture

`drm-custom-colorfix` automatically detects your desktop environment and routes calibration through the appropriate subsystem:

```
                          ┌─────────────────────────────┐
                          │   drm-custom-colorfix CLI   │
                          │   (-t 8000 -b 0.9 --backend)│
                          └──────────────┬──────────────┘
                                         │
                         ┌───────────────┴───────────────┐
                         ▼                               ▼
               [ GNOME / Wayland ]             [ COSMIC / DRM / TTY ]
                         │                               │
             Generate ICC Profile (v2.4)       Calculate CRTC Gamma Table
             with VCGT (Gamma Table) Tag                 │
                         │                     DRM Mode SETGAMMA Ioctl
                         ▼                     (with VT auto-bounce if locked)
                colord / gsd-color                       │
                         │                               ▼
                         ▼                       GPU Hardware CRTC
                  Mutter Compositor               Display Output
                         │
                         ▼
                   Display Output
```

### 1. GNOME / Wayland Backend (`--backend gnome`)
- **How it Works**: Generates a standard, standards-compliant ICC v2.4 display color profile containing the calculated hardware gamma lookup table in the `vcgt` (Video Card Gamma Table) tag. The profile is registered in `colord` and activated on the target display. GNOME Shell / `gsd-color` / Mutter immediately loads the VCGT LUT into the compositor's color pipeline.
- **Benefits**:
  - **No root permissions required** for manual adjustment.
  - **No DRM master conflicts** or TTY switches.
  - Smooth, flicker-free, persistent display calibration across sessions.
  - Fully compatible with GNOME 40 through GNOME 50 / 51 on Fedora 44.

### 2. DRM / COSMIC Backend (`--backend drm`)
- **How it Works**: Communicates directly with the Linux kernel's Direct Rendering Manager (DRM) subsystem via raw ioctls (`GETRESOURCES`, `GETCRTC`, `SETGAMMA`) to program the GPU's CRTC hardware gamma lookup tables directly.
- **COSMIC Auto-Bounce (`AUTO_ACTIVATE=1`)**: When `cosmic-comp` holds DRM master on session start, the daemon executes a 0.05s background micro-bounce (`VT 1 -> VT 2 -> VT 1`) to flash the calibrated gamma table directly into the GPU hardware.

---

## System Requirements & Dependencies

### Fedora / RHEL
- For **GNOME / Wayland**: `colord` (provides `/usr/bin/colormgr`, usually pre-installed on Fedora Workstation).
- For building from source: `cargo`, `rustc`, `make`, `gcc`, `rpm-build`, `systemd-rpm-macros`.

Install build dependencies:
```bash
sudo dnf install cargo rust make gcc colord
```

---

## Configuration (`/etc/default/drm-custom-colorfix.conf`)

```ini
# /etc/default/drm-custom-colorfix.conf
# Configuration for DRM Custom Colorfix

# Target color temperature in Kelvin (1000-10000). Default: 8000
TEMPERATURE=8000

# Brightness multiplier (0.1 - 1.0). Default: 1.0
BRIGHTNESS=1.0

# Backend: 'auto' (detect GNOME vs COSMIC/DRM), 'gnome' (colord/ICC), or 'drm' (direct CRTC)
BACKEND=auto

# Optional connector filter (e.g. eDP-1, HDMI-A-1)
# CONNECTOR="eDP-1"

# Automatic VT micro-switch for DRM/COSMIC backend (1 = enabled, 0 = disabled)
AUTO_ACTIVATE=1

# DRM device to calibrate for DRM backend (leave commented out to auto-detect)
# DEVICE="/dev/dri/card1"
```

### Configuration Options Reference

| Key | Default | Description |
| :--- | :--- | :--- |
| `TEMPERATURE` | `8000` | Target color temperature in Kelvin (1000 – 10000). 8000K–8200K is ideal for correcting warm/yellow replacement displays. |
| `BRIGHTNESS` | `1.0` | Brightness multiplier from `0.1` to `1.0`. |
| `BACKEND` | `auto` | Backend selection: `auto` (auto-detect), `gnome` (colord/ICC), or `drm` (direct CRTC ioctl). |
| `CONNECTOR` | *(empty)* | Optional connector/display filter (e.g. `eDP-1`, `HDMI-A-1`). |
| `AUTO_ACTIVATE` | `1` | Automatically switches VT for 0.05s on startup in DRM/COSMIC mode. |
| `DEVICE` | *(auto)* | Path to DRM card (e.g. `/dev/dri/card1`). Omit to auto-detect all GPUs. |
| `GAMMA_SIZE` | `0` | Override hardware gamma LUT size (`0` = auto-detect). |
| `VERBOSE` | `0` | Enable verbose logging (`1` = enabled, `0` = disabled). |

---

## CLI Usage

### Quick Adjustments (Auto-Detect Backend)

```bash
# Set screen temperature to 8000K
drm-custom-colorfix -t 8000

# Set temperature to 8200K with 90% brightness
drm-custom-colorfix -t 8200 -b 0.9

# List all detected displays
drm-custom-colorfix -l

# Reset displays to standard neutral white (6500K / original base profile)
drm-custom-colorfix -r
```

### Explicit GNOME Backend

```bash
# List all color-managed displays known to colord
drm-custom-colorfix -B gnome -l

# Apply 8200K temperature to all GNOME displays
drm-custom-colorfix -B gnome -t 8200

# Apply 8200K only to the laptop built-in display (eDP-1)
drm-custom-colorfix -B gnome -C eDP-1 -t 8200

# Reset GNOME display calibration and restore original EDID profile
drm-custom-colorfix -B gnome -r
```

### Explicit DRM / COSMIC Backend

```bash
# List all DRM devices, CRTCs, and connectors
drm-custom-colorfix -B drm -l

# Apply 8000K directly via DRM ioctls (requires root/video group)
sudo drm-custom-colorfix -B drm -t 8000

# Target a specific DRM device
sudo drm-custom-colorfix -B drm -d /dev/dri/card1 -t 8000

# Apply once from config file and exit
sudo drm-custom-colorfix --apply -c /etc/default/drm-custom-colorfix.conf
```

---

## Installation & System Service

### 1. Build and Install Binary / Service

```bash
# Build release binary
make build

# Install binary, systemd service, and configuration
sudo make install
```

### 2. Enable Service (for Background Startup)

```bash
# Enable and start systemd daemon
sudo systemctl enable --now drm-custom-colorfix

# Check service status
sudo systemctl status drm-custom-colorfix
```

---

## Building & Testing from Source

```bash
# Run full unit test suite
cargo test

# Check code formatting
cargo fmt --check

# Build release binary
cargo build --release

# Inspect generated ICC profile with cd-iccdump (if colord is installed)
cd-iccdump ~/.local/share/icc/drm-colortemp-*.icc
```

---

## Troubleshooting

### GNOME backend: "colormgr utility not found"
- Install `colord`: `sudo dnf install colord`
- Verify colord is running: `busctl --system status org.freedesktop.ColorManager`

### DRM backend: "Cannot open /dev/dri/cardX: Permission denied"
- Run with `sudo` or add your user to the `video` group: `sudo usermod -aG video $USER`.

### How to completely remove custom profiles
- Run `drm-custom-colorfix -r` to restore your base profile.
- Generated profiles are stored in `~/.local/share/icc/drm-colortemp-*.icc` and can be deleted anytime.

---

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).

---

## Credits & Acknowledgments

- Upstream project: [jjo/drm-colortemp](https://github.com/jjo/drm-colortemp).
- Color temperature algorithm based on Tanner Helland's blackbody conversion formulas.
- GNOME ICC VCGT workflow inspired by `gnome-gamma-tool`.
