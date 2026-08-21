//! GNOME / Wayland backend via colord and ICC profiles with VCGT tags.
//!
//! Under GNOME/Wayland, Mutter owns the DRM master and manages hardware CRTC
//! gamma ramps via colord and gnome-settings-daemon (gsd-color). This backend
//! generates standard ICC profiles containing the calculated gamma LUT inside
//! a VCGT (Video Card Gamma Table) tag and activates them through colord.

use crate::icc;
use crate::temperature;
use log::{debug, info, warn};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone)]
pub struct ColordDisplayDevice {
    pub object_path: String,
    pub device_id: String,
    pub model: String,
    pub vendor: String,
    #[allow(dead_code)]
    pub serial: String,
    pub connector: String,
    pub enabled: bool,
    pub default_profile_id: Option<String>,
    pub default_profile_filename: Option<String>,
    pub profiles: Vec<(String, String)>, // (profile_id, filename)
}

/// Check whether the GNOME/colord backend is available.
pub fn is_available() -> bool {
    // Check if colormgr binary exists and colord is responding
    which_colormgr().is_some() && colord_running()
}

fn which_colormgr() -> Option<PathBuf> {
    for dir in &["/usr/bin", "/usr/local/bin", "/bin"] {
        let p = Path::new(dir).join("colormgr");
        if p.exists() {
            return Some(p);
        }
    }
    // Fall back to PATH search via standard command
    if let Ok(out) = Command::new("which").arg("colormgr").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(PathBuf::from(s));
            }
        }
    }
    None
}

fn colord_running() -> bool {
    let Some(colormgr) = which_colormgr() else {
        return false;
    };
    let Ok(out) = Command::new(colormgr).arg("get-devices").output() else {
        return false;
    };
    out.status.success()
}

/// Query colord for all connected display devices.
pub fn get_display_devices() -> Result<Vec<ColordDisplayDevice>, String> {
    let colormgr = which_colormgr().ok_or_else(|| {
        "colormgr utility not found. Please install colord ('sudo dnf install colord')".to_string()
    })?;

    let output = Command::new(&colormgr)
        .arg("get-devices-by-kind")
        .arg("display")
        .output()
        .map_err(|e| format!("Failed to execute colormgr: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "colormgr get-devices-by-kind display failed: {err}"
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_colormgr_devices(&stdout))
}

/// Parse the raw text output from `colormgr get-devices` into structured device records.
pub fn parse_colormgr_devices(output: &str) -> Vec<ColordDisplayDevice> {
    let mut devices = Vec::new();
    let blocks = output
        .split("Object Path:")
        .filter(|b| !b.trim().is_empty());

    for block in blocks {
        let mut obj_path = String::new();
        let mut dev_id = String::new();
        let mut model = String::new();
        let mut vendor = String::new();
        let mut serial = String::new();
        let mut connector = String::new();
        let mut enabled = true;
        let mut profiles = Vec::new();

        let full_block = format!("Object Path:{block}");
        for line in full_block.lines() {
            let line = line.trim();
            if let Some(val) = line.strip_prefix("Object Path:") {
                obj_path = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("Device ID:") {
                dev_id = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("Model:") {
                model = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("Vendor:") {
                vendor = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("Serial:") {
                serial = val.trim().to_string();
            } else if let Some(val) = line.strip_prefix("Enabled:") {
                enabled = !val.trim().eq_ignore_ascii_case("no");
            } else if let Some(val) = line.strip_prefix("Metadata:") {
                let meta = val.trim();
                if let Some(conn) = meta.strip_prefix("XRANDR_name=") {
                    connector = conn.trim().to_string();
                }
            } else if line.starts_with("Profile ") {
                if let Some((_, rest)) = line.split_once(':') {
                    let prof_id = rest.trim().to_string();
                    profiles.push((prof_id, String::new()));
                }
            } else if line.starts_with('/') && line.ends_with(".icc") {
                if let Some(last) = profiles.last_mut() {
                    last.1 = line.trim().to_string();
                }
            }
        }

        // If connector wasn't found in metadata, try extracting from Device ID (e.g. xrandr-eDP-1)
        if connector.is_empty() {
            if let Some(rest) = dev_id.strip_prefix("xrandr-") {
                connector = rest.split('-').next().unwrap_or(rest).to_string();
            }
        }

        if !obj_path.is_empty() || !dev_id.is_empty() {
            let default_profile_id = profiles.first().map(|p| p.0.clone());
            let default_profile_filename = profiles.first().map(|p| p.1.clone());

            devices.push(ColordDisplayDevice {
                object_path: obj_path,
                device_id: dev_id,
                model,
                vendor,
                serial,
                connector,
                enabled,
                default_profile_id,
                default_profile_filename,
                profiles,
            });
        }
    }

    devices
}

/// List all color-managed displays known to GNOME/colord.
pub fn list_displays() -> Result<(), String> {
    println!("GNOME / colord Color Management Displays:\n");
    let devices = get_display_devices()?;

    if devices.is_empty() {
        println!("  No display devices found registered with colord.");
        println!("  Ensure GNOME session (Mutter / gsd-color) is running.");
        return Ok(());
    }

    for (i, dev) in devices.iter().enumerate() {
        println!("  Display {}:", i + 1);
        println!("    Device ID:  {}", dev.device_id);
        println!(
            "    Connector:  {}",
            if dev.connector.is_empty() {
                "auto"
            } else {
                &dev.connector
            }
        );
        if !dev.model.is_empty() || !dev.vendor.is_empty() {
            println!("    Model:      {} {}", dev.vendor, dev.model);
        }
        println!("    Enabled:    {}", if dev.enabled { "Yes" } else { "No" });
        if let Some(def_id) = &dev.default_profile_id {
            println!("    Active Profile ID: {}", def_id);
        }
        if let Some(def_file) = &dev.default_profile_filename {
            if !def_file.is_empty() {
                println!("    Active File:       {}", def_file);
            }
        }
        if dev.profiles.len() > 1 {
            println!("    All Profiles ({} total):", dev.profiles.len());
            for (pid, pfile) in &dev.profiles {
                if !pfile.is_empty() {
                    println!("      - {pid} ({pfile})");
                } else {
                    println!("      - {pid}");
                }
            }
        }
        println!();
    }

    Ok(())
}

/// Apply a color temperature and brightness to GNOME display(s).
pub fn apply_temperature(
    temp: u32,
    brightness: f64,
    connector_filter: &str,
) -> Result<u32, String> {
    let colormgr = which_colormgr().ok_or_else(|| {
        "colormgr utility not found. Please install colord ('sudo dnf install colord')".to_string()
    })?;

    let devices = get_display_devices()?;
    if devices.is_empty() {
        return Err("No display devices found in colord".to_string());
    }

    let icc_dir = get_icc_storage_dir()?;
    fs::create_dir_all(&icc_dir)
        .map_err(|e| format!("Failed to create ICC directory {}: {e}", icc_dir.display()))?;

    // Generate gamma LUTs
    let (r, g, b) = temperature::generate_gamma_luts(256, temp, brightness);

    let mut applied_count = 0u32;

    for dev in &devices {
        if !matches_filter(dev, connector_filter) {
            debug!(
                "Skipping device {} (does not match filter '{connector_filter}')",
                dev.device_id
            );
            continue;
        }

        let sanitize_id = sanitize_filename(&dev.device_id);
        let profile_filename = format!("drm-colortemp-{sanitize_id}-{temp}K.icc");
        let profile_path = icc_dir.join(&profile_filename);

        let desc = format!(
            "DRM ColorFix {}K ({:.0}% bright) - {}",
            temp,
            brightness * 100.0,
            if !dev.connector.is_empty() {
                &dev.connector
            } else {
                &dev.model
            }
        );

        let icc_data = icc::create_icc_profile_with_vcgt(&desc, &r, &g, &b);
        fs::write(&profile_path, &icc_data).map_err(|e| {
            format!(
                "Failed to write ICC profile to {}: {e}",
                profile_path.display()
            )
        })?;

        info!("Wrote ICC profile with VCGT to {}", profile_path.display());

        // Import profile into colord
        let import_out = run_cmd(
            &colormgr,
            &["import-profile", profile_path.to_str().unwrap()],
        );
        let profile_id = match extract_profile_id(&import_out) {
            Some(id) => id,
            None => {
                // If import didn't return an ID directly, search for it
                find_profile_by_filename(&colormgr, profile_path.to_str().unwrap())
                    .unwrap_or_else(|| profile_path.to_string_lossy().into_owned())
            }
        };

        info!("Profile registered in colord: {profile_id}");

        // Associate profile with device
        let target_dev = if !dev.device_id.is_empty() {
            &dev.device_id
        } else {
            &dev.object_path
        };

        let add_res = run_cmd(&colormgr, &["device-add-profile", target_dev, &profile_id]);
        debug!("device-add-profile result: {add_res:?}");

        // Make profile default
        let make_res = run_cmd(
            &colormgr,
            &["device-make-profile-default", target_dev, &profile_id],
        );
        if make_res.status.success() {
            info!(
                "Applied {}K (brightness {:.2}) to GNOME display {} via colord",
                temp, brightness, dev.device_id
            );
            applied_count += 1;
        } else {
            let err = String::from_utf8_lossy(&make_res.stderr);
            warn!("Failed to make profile default for {target_dev}: {err}");
        }
    }

    if applied_count == 0 {
        if !connector_filter.is_empty() {
            return Err(format!(
                "No displays matched connector/device filter '{connector_filter}'"
            ));
        }
        return Err("Failed to apply ICC profile to any display in colord".to_string());
    }

    Ok(applied_count)
}

/// Reset display(s) to original calibration profile or 6500K neutral.
pub fn reset(connector_filter: &str) -> Result<(), String> {
    let colormgr = which_colormgr().ok_or_else(|| {
        "colormgr utility not found. Please install colord ('sudo dnf install colord')".to_string()
    })?;

    let devices = get_display_devices()?;
    if devices.is_empty() {
        return Err("No display devices found in colord".to_string());
    }

    for dev in &devices {
        if !matches_filter(dev, connector_filter) {
            continue;
        }

        let target_dev = if !dev.device_id.is_empty() {
            &dev.device_id
        } else {
            &dev.object_path
        };

        // Find non-drm-colortemp original profile (e.g. edid-*.icc or system profile)
        let original_profile = dev
            .profiles
            .iter()
            .find(|(id, file)| !id.contains("drm-colortemp") && !file.contains("drm-colortemp"));

        if let Some((orig_id, orig_file)) = original_profile {
            info!("Restoring original profile for {target_dev}: {orig_id} ({orig_file})");
            let _ = run_cmd(
                &colormgr,
                &["device-make-profile-default", target_dev, orig_id],
            );
        } else {
            // No original profile exists; apply neutral 6500K 1.0 brightness
            info!("No base profile found for {target_dev}, applying neutral 6500K profile");
            apply_temperature(6500, 1.0, connector_filter)?;
        }
    }

    info!("GNOME displays reset successfully");
    Ok(())
}

fn matches_filter(dev: &ColordDisplayDevice, filter: &str) -> bool {
    if filter.is_empty() {
        return true;
    }
    let filter_lower = filter.to_ascii_lowercase();
    dev.connector.to_ascii_lowercase() == filter_lower
        || dev.device_id.to_ascii_lowercase().contains(&filter_lower)
        || dev.model.to_ascii_lowercase().contains(&filter_lower)
        || dev.vendor.to_ascii_lowercase().contains(&filter_lower)
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn get_icc_storage_dir() -> Result<PathBuf, String> {
    if let Ok(home) = std::env::var("HOME") {
        Ok(PathBuf::from(home).join(".local/share/icc"))
    } else if let Ok(user) = std::env::var("USER") {
        Ok(PathBuf::from(format!("/home/{user}/.local/share/icc")))
    } else {
        Ok(PathBuf::from("/var/lib/colord/icc"))
    }
}

fn run_cmd(program: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .output()
        .unwrap_or_else(|_| Output {
            status: std::process::ExitStatus::default(),
            stdout: Vec::new(),
            stderr: b"Command execution failed".to_vec(),
        })
}

fn extract_profile_id(output: &Output) -> Option<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(id) = line.strip_prefix("Profile ID:") {
            return Some(id.trim().to_string());
        }
    }
    None
}

fn find_profile_by_filename(colormgr: &Path, filename: &str) -> Option<String> {
    let out = Command::new(colormgr)
        .arg("find-profile-by-filename")
        .arg(filename)
        .output()
        .ok()?;
    if out.status.success() {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let line = line.trim();
            if let Some(id) = line.strip_prefix("Profile ID:") {
                return Some(id.trim().to_string());
            }
            if let Some(path) = line.strip_prefix("Object Path:") {
                return Some(path.trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_colormgr_devices() {
        let raw = r#"
Object Path:   /org/freedesktop/ColorManager/devices/xrandr_BOE_NE156FHM_NX6_0x00000000_fus0g_1000
Owner:         fus0g
Type:          display
Enabled:       Yes
Embedded:      Yes
Model:         NE156FHM-NX6
Vendor:        BOE
Serial:        0x00000000
Device ID:     xrandr-BOE-NE156FHM-NX6-0x00000000
Profile 1:     icc-a64f75aa6a182cefb71d5209cc5924be
               /home/fus0g/.local/share/icc/edid-b4b381880d033dd171a938aa8455e7c9.icc
Metadata:      XRANDR_name=eDP-1
"#;

        let devices = parse_colormgr_devices(raw);
        assert_eq!(devices.len(), 1);
        let d = &devices[0];
        assert_eq!(d.device_id, "xrandr-BOE-NE156FHM-NX6-0x00000000");
        assert_eq!(d.connector, "eDP-1");
        assert_eq!(d.model, "NE156FHM-NX6");
        assert_eq!(d.vendor, "BOE");
        assert!(d.enabled);
        assert_eq!(
            d.default_profile_id.as_deref(),
            Some("icc-a64f75aa6a182cefb71d5209cc5924be")
        );
        assert_eq!(
            d.default_profile_filename.as_deref(),
            Some("/home/fus0g/.local/share/icc/edid-b4b381880d033dd171a938aa8455e7c9.icc")
        );
    }

    #[test]
    fn test_matches_filter() {
        let dev = ColordDisplayDevice {
            object_path: "/org/freedesktop/ColorManager/devices/dev1".into(),
            device_id: "xrandr-BOE-NE156FHM-NX6-0x00000000".into(),
            model: "NE156FHM-NX6".into(),
            vendor: "BOE".into(),
            serial: "0".into(),
            connector: "eDP-1".into(),
            enabled: true,
            default_profile_id: None,
            default_profile_filename: None,
            profiles: vec![],
        };

        assert!(matches_filter(&dev, ""));
        assert!(matches_filter(&dev, "eDP-1"));
        assert!(matches_filter(&dev, "edp-1"));
        assert!(matches_filter(&dev, "BOE"));
        assert!(matches_filter(&dev, "NE156FHM"));
        assert!(!matches_filter(&dev, "HDMI-1"));
    }
}
