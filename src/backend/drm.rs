//! Direct Linux DRM / CRTC backend.
//!
//! Directly programs hardware CRTC gamma lookup tables via raw Linux DRM ioctls.
//! Used on COSMIC Desktop, TTYs, or standalone display setups where direct DRM
//! access is available.

use crate::device;
use crate::drm;
use crate::temperature;
use log::{debug, error, info, warn};

/// Apply temperature and brightness via direct DRM CRTC SETGAMMA ioctls.
pub fn apply_temperature(
    temp: u32,
    brightness: f64,
    device_path: &str,
    connector_filter: &str,
) -> Result<u32, String> {
    info!("Setting {temp}K, brightness {brightness:.2} via DRM");

    let dev = device::open_device(device_path)
        .map_err(|e| format!("Cannot open DRM device {device_path}: {e}"))?;

    if dev.path() != device_path {
        info!(
            "Using device: {} (preferred {} unusable)",
            dev.path(),
            device_path
        );
    }

    device::try_become_master(&dev);

    let res = drm::get_resources(dev.fd())
        .map_err(|e| format!("GETRESOURCES failed on {}: {e}", dev.path()))?;

    if res.crtcs.is_empty() {
        return Err(format!("No CRTCs on {}", dev.path()));
    }

    let mask = drm::matching_crtc_mask(dev.fd(), &res, connector_filter);
    let mut success = 0u32;

    for (i, &crtc_id) in res.crtcs.iter().enumerate() {
        if mask & (1u32 << i) == 0 {
            continue;
        }

        let info = match drm::get_crtc(dev.fd(), crtc_id) {
            Ok(c) => c,
            Err(e) => {
                error!("GETCRTC {crtc_id}: {e}");
                continue;
            }
        };

        if !info.mode_valid {
            debug!("Skip inactive CRTC {crtc_id}");
            continue;
        }

        if info.gamma_size == 0 {
            warn!("CRTC {crtc_id} has no gamma support");
            continue;
        }

        let (r, g, b) =
            temperature::generate_gamma_luts(info.gamma_size as usize, temp, brightness);

        match drm::set_gamma(dev.fd(), crtc_id, &r, &g, &b) {
            Ok(()) => {
                info!("Applied to CRTC {crtc_id}");
                success += 1;
            }
            Err(e) => error!("SETGAMMA CRTC {crtc_id}: {e}"),
        }
    }

    if success == 0 {
        return Err("Failed to apply gamma to any CRTC".to_string());
    }

    info!("Successfully adjusted {success} DRM display(s)");
    Ok(success)
}

/// Reset DRM CRTC gamma ramps to default (6500K, 1.0 brightness).
pub fn reset(device_path: &str, connector_filter: &str) -> Result<(), String> {
    info!("Resetting DRM display to 6500K neutral");
    apply_temperature(6500, 1.0, device_path, connector_filter).map(|_| ())
}

/// List DRM devices, CRTCs, and connectors.
pub fn list_displays(device_path: &str) -> Result<(), String> {
    println!("Available DRM devices:");
    let devices = device::find_all_devices(8);
    if devices.is_empty() {
        println!("  No DRM devices found");
    } else {
        for (i, p) in devices.iter().enumerate() {
            let state = if device::device_accessible(p) {
                "accessible"
            } else {
                "not accessible"
            };
            println!("  {}: {p} ({state})", i + 1);
        }
    }

    println!();
    let dev = match device::open_device(device_path) {
        Ok(d) => d,
        Err(e) => {
            println!("Cannot open {device_path}: {e}");
            return Ok(());
        }
    };

    println!("Device {} details:", dev.path());
    match drm::get_resources(dev.fd()) {
        Ok(res) => {
            if res.crtcs.is_empty() {
                println!("  No CRTCs");
            } else {
                println!("  CRTCs:");
                for (i, &id) in res.crtcs.iter().enumerate() {
                    match drm::get_crtc(dev.fd(), id) {
                        Ok(info) => println!(
                            "    {}: ID={id}  gamma_size={}  mode_valid={}",
                            i, info.gamma_size, info.mode_valid
                        ),
                        Err(e) => println!("    {}: ID={id}  (GETCRTC failed: {e})", i),
                    }
                }
            }
            if !res.connectors.is_empty() {
                println!("  Connectors:");
                for &id in &res.connectors {
                    match drm::get_connector(dev.fd(), id) {
                        Ok(c) => {
                            let (long, short) =
                                drm::connector_names(c.connector_type, c.connector_type_id);
                            println!(
                                "    ID={id}  {long} (alias {short})  encoder={}",
                                c.encoder_id
                            );
                        }
                        Err(e) => println!("    ID={id}  (GETCONNECTOR failed: {e})"),
                    }
                }
            }
        }
        Err(e) => println!("  GETRESOURCES failed: {e}"),
    }

    Ok(())
}
