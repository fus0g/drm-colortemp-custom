//! DRM Custom Colorfix - Display color temperature and calibration utility.
//!
//! Supports two backends:
//! 1. GNOME / Wayland backend via colord & ICC VCGT profiles.
//! 2. Direct DRM backend via CRTC gamma ioctls (for COSMIC, TTYs, or root services).

mod backend;
mod config;
mod daemon;
mod device;
mod drm;
mod icc;
mod temperature;
mod vt;

use clap::{CommandFactory, Parser};
use log::{error, info};
use nix::unistd::geteuid;
use std::process::ExitCode;

const DEFAULT_DEVICE: &str = "/dev/dri/card1";
const DEFAULT_DAEMON_CONFIG: &str = "/etc/default/drm-custom-colorfix.conf";

#[derive(Parser, Debug)]
#[command(
    name = "drm-custom-colorfix",
    author,
    version,
    about = "Adjust display color temperature and calibration",
    long_about = "A tool for adjusting display color temperature and white point.\n\
    Supports GNOME / Wayland (via colord + ICC VCGT profiles) and direct DRM (COSMIC/TTY).\n\n\
    Examples:\n  \
    drm-custom-colorfix -t 8200                 # Set temperature to 8200K (auto-detect backend)\n  \
    drm-custom-colorfix -t 8200 -b 0.9          # 8200K, 90% brightness\n  \
    drm-custom-colorfix -B gnome -t 8200        # Explicitly use GNOME backend\n  \
    drm-custom-colorfix -B drm -t 8200          # Explicitly use direct DRM backend\n  \
    drm-custom-colorfix -l                      # List available displays\n  \
    drm-custom-colorfix -r                      # Reset display to standard neutral\n  \
    drm-custom-colorfix --daemon -c /etc/default/drm-custom-colorfix.conf"
)]
struct Args {
    /// Color temperature in Kelvin (1000-10000)
    #[arg(short = 't', long)]
    temperature: Option<u32>,

    /// Brightness multiplier (0.1-1.0)
    #[arg(short = 'b', long)]
    brightness: Option<f64>,

    /// Calibration backend ('auto', 'gnome', or 'drm')
    #[arg(short = 'B', long, default_value = "auto")]
    backend: String,

    /// Connector / display filter (e.g. 'eDP-1', 'DP-1', 'HDMI-A-1')
    #[arg(short = 'C', long, default_value = "")]
    connector: String,

    /// DRM device path (used by direct DRM backend)
    #[arg(short = 'd', long, default_value = DEFAULT_DEVICE)]
    device: String,

    /// Reset to default (6500K neutral / original profile)
    #[arg(short = 'r', long)]
    reset: bool,

    /// List available displays
    #[arg(short = 'l', long)]
    list: bool,

    /// Run as daemon (background service)
    #[arg(short = 'D', long)]
    daemon: bool,

    /// Daemon config file (only used with --daemon)
    #[arg(short = 'c', long, default_value = DEFAULT_DAEMON_CONFIG)]
    config: String,

    /// Apply temperature once from config and exit (for boot/startup)
    #[arg(short = 'a', long)]
    apply: bool,

    /// Verbose output
    #[arg(short = 'v', long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let args = Args::parse();
    init_logging(args.verbose);

    let backend_kind = match backend::BackendKind::parse(&args.backend) {
        Some(b) => b,
        None => {
            eprintln!(
                "Error: Unknown backend '{}'. Supported options: 'auto', 'gnome', 'drm'",
                args.backend
            );
            return ExitCode::from(1);
        }
    };
    let active_backend = backend::resolve_backend(backend_kind);

    if args.apply {
        info!("Applying temperature once from config: {}", args.config);
        return match daemon::apply_from_config(&args.config) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                error!("Apply from config failed: {e}");
                eprintln!("Error: {e}");
                ExitCode::from(1)
            }
        };
    }

    if args.daemon {
        if !geteuid().is_root() {
            eprintln!("Error: --daemon must run as root (DRM ioctls require it)");
            return ExitCode::from(1);
        }
        info!("Starting daemon with config: {}", args.config);
        return match daemon::run(&args.config) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                error!("Daemon error: {e}");
                eprintln!("Error: {e}");
                ExitCode::from(1)
            }
        };
    }

    if args.reset {
        info!("Resetting display to defaults (backend: {active_backend})");
        return match active_backend {
            backend::BackendKind::Gnome => match backend::gnome::reset(&args.connector) {
                Ok(()) => {
                    println!("Successfully reset GNOME display calibration.");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    error!("GNOME reset failed: {e}");
                    eprintln!("Error: {e}");
                    ExitCode::from(1)
                }
            },
            backend::BackendKind::Drm | backend::BackendKind::Auto => {
                match backend::drm::reset(&args.device, &args.connector) {
                    Ok(()) => {
                        println!("Successfully reset DRM display gamma.");
                        ExitCode::SUCCESS
                    }
                    Err(e) => {
                        error!("DRM reset failed: {e}");
                        eprintln!("Error: {e}");
                        ExitCode::from(1)
                    }
                }
            }
        };
    }

    if args.list {
        return match active_backend {
            backend::BackendKind::Gnome => match backend::gnome::list_displays() {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    error!("GNOME list displays failed: {e}");
                    eprintln!("Error: {e}");
                    ExitCode::from(1)
                }
            },
            backend::BackendKind::Drm | backend::BackendKind::Auto => {
                match backend::drm::list_displays(&args.device) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        error!("DRM list displays failed: {e}");
                        eprintln!("Error: {e}");
                        ExitCode::from(1)
                    }
                }
            }
        };
    }

    if let Some(temp) = args.temperature {
        if !(1000..=10000).contains(&temp) {
            eprintln!("Temperature must be between 1000 and 10000K");
            return ExitCode::from(1);
        }
        let brightness = args.brightness.unwrap_or(1.0);
        if !(0.1..=1.0).contains(&brightness) {
            eprintln!("Brightness must be between 0.1 and 1.0");
            return ExitCode::from(1);
        }
        return apply_temperature_cli(
            temp,
            brightness,
            active_backend,
            &args.device,
            &args.connector,
        );
    }

    if let Some(brightness) = args.brightness {
        if !(0.1..=1.0).contains(&brightness) {
            eprintln!("Brightness must be between 0.1 and 1.0");
            return ExitCode::from(1);
        }
        return apply_temperature_cli(
            6500,
            brightness,
            active_backend,
            &args.device,
            &args.connector,
        );
    }

    let _ = Args::command().print_help();
    println!();
    ExitCode::SUCCESS
}

fn init_logging(verbose: bool) {
    let level = if verbose {
        log::LevelFilter::Debug
    } else {
        log::LevelFilter::Info
    };
    let _ = env_logger::Builder::from_default_env()
        .filter_level(level)
        .try_init();
}

fn apply_temperature_cli(
    temp: u32,
    brightness: f64,
    backend: backend::BackendKind,
    device_path: &str,
    connector: &str,
) -> ExitCode {
    match backend {
        backend::BackendKind::Gnome => {
            match backend::gnome::apply_temperature(temp, brightness, connector) {
                Ok(count) => {
                    println!("Successfully applied {temp}K (brightness {brightness:.2}) to {count} GNOME display(s).");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    error!("GNOME apply failed: {e}");
                    eprintln!("Error: {e}");
                    eprintln!("\nTips:");
                    eprintln!("  - Ensure colord is running and GNOME session is active.");
                    eprintln!("  - Check displays with: drm-custom-colorfix -B gnome -l");
                    eprintln!("  - To force the direct DRM backend instead, run: drm-custom-colorfix -B drm -t {temp}");
                    ExitCode::from(1)
                }
            }
        }
        backend::BackendKind::Drm | backend::BackendKind::Auto => {
            match backend::drm::apply_temperature(temp, brightness, device_path, connector) {
                Ok(count) => {
                    println!("Successfully applied {temp}K (brightness {brightness:.2}) to {count} DRM display(s).");
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    error!("{e}");
                    eprintln!("Error: {e}");
                    eprintln!("\nTips:");
                    eprintln!(
                        "  - If running under GNOME/Wayland, use the GNOME backend: -B gnome"
                    );
                    eprintln!("  - Try running with sudo, or add your user to the 'video' group.");
                    eprintln!("  - Or specify a device with: -d /dev/dri/cardX");
                    ExitCode::from(1)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temperature_validation() {
        assert!(temperature::temp_to_rgb(1000).0 >= 0.0);
        assert!(temperature::temp_to_rgb(10000).0 >= 0.0);
    }

    #[test]
    fn test_gamma_lut_sizes() {
        let (r, g, b) = temperature::generate_gamma_luts(256, 6500, 1.0);
        assert_eq!(r.len(), 256);
        assert_eq!(g.len(), 256);
        assert_eq!(b.len(), 256);
    }

    #[test]
    fn test_brightness_range_check() {
        assert!((0.1..=1.0).contains(&0.5));
        assert!(!(0.1..=1.0).contains(&0.05));
        assert!(!(0.1..=1.0).contains(&1.5));
    }
}
