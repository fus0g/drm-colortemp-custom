# drm-custom-colorfix

A minimal, fast, and standalone DRM color temperature and display calibration utility for Linux Wayland compositors (specifically **COSMIC Desktop Environment** / `cosmic-comp`), designed to correct screen color balance, fix warm/yellowish tints on replacement laptop panels, and maintain a constant, calibrated display white-point.

> **Attribution:** This project is a streamlined fork of [jjo/drm-colortemp](https://github.com/jjo/drm-colortemp). It has been stripped of all unnecessary complexity (legacy C codebase, day/night schedules, solar calculators, desktop notifications, separate applets) to provide a focused, single-binary Rust tool with **automatic background boot-time calibration** and **Fedora RPM packaging**.

---

## How It Works

1. **Automatic Background Calibration (`AUTO_ACTIVATE=1`)**:
   - When your system starts or you log in, `drm-custom-colorfix` ensures your configured display temperature is applied.
   - If the Wayland compositor holds a lock on the active virtual terminal (VT), the daemon performs a background 0.05s micro-bounce (`VT 1 -> VT 2 -> VT 1`) to program the GPU hardware gamma curves directly.
   - Zero key combinations or manual intervention needed.

2. **Live Reload**:
   - Whenever `/etc/default/drm-custom-colorfix.conf` is edited, changes apply immediately via inotify.

---

## Configuration (`/etc/default/drm-custom-colorfix.conf`)

The configuration is minimal and straightforward:

```ini
# /etc/default/drm-custom-colorfix.conf

# Target color temperature in Kelvin (1000-10000). Default: 8000
TEMPERATURE=8000

# Brightness multiplier (0.1 - 1.0). Default: 1.0
BRIGHTNESS=1.0

# Automatic VT micro-switch on boot/session start (1 = enabled, 0 = disabled)
AUTO_ACTIVATE=1

# DRM device to calibrate (leave commented out to auto-detect all connected GPUs)
# DEVICE="/dev/dri/card1"
```

---

## Quick Start (Fedora / RHEL)

### 1. Build and Install RPM

```bash
# Build RPM
make rpm

# Install or upgrade package
sudo dnf install ./build-rpm/RPMS/x86_64/drm-custom-colorfix-2.1.0-2.fc44.x86_64.rpm
```

### 2. Copy Configuration & Start Service

```bash
sudo cp ./drm-custom-colorfix.conf /etc/default/drm-custom-colorfix.conf
sudo systemctl enable --now drm-custom-colorfix
```

---

## CLI Usage

```bash
# Set temperature to 8000K manually
sudo drm-custom-colorfix -t 8000

# Set temperature with 85% brightness
sudo drm-custom-colorfix -t 8000 -b 0.85

# List detected DRM cards and displays
drm-custom-colorfix -l

# Reset to default 6500K neutral white
sudo drm-custom-colorfix -r
```

---

## License

GPL-3.0-or-later — see [LICENSE](LICENSE).

---

## Credits & Acknowledgments

- Forked from [jjo/drm-colortemp](https://github.com/jjo/drm-colortemp).
- Color temperature algorithm based on Tanner Helland's blackbody conversion formulas.
