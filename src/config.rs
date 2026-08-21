//! Configuration parsing for DRM Custom Colorfix.

use crate::device;
use log::{info, warn};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use thiserror::Error;

pub const MAX_DEVICES: usize = 8;
pub const GAMMA_SIZE_MIN: u32 = 64;
pub const GAMMA_SIZE_MAX: u32 = 4096;

#[derive(Debug, Clone)]
pub struct Config {
    pub devices: Vec<String>,
    pub temperature: u32,
    pub brightness: f64,
    pub check_interval: u32,
    pub verbose: bool,
    pub connector: String,
    pub gamma_size: u32,
    pub auto_activate: bool,
    pub monitor_tty: i32,
    pub backend: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            temperature: 8000,
            brightness: 1.0,
            check_interval: 1,
            verbose: false,
            connector: String::new(),
            gamma_size: 0,
            auto_activate: true,
            monitor_tty: 3,
            backend: "auto".to_string(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to open config file {path}: {source}")]
    Open {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to read config file {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
    let path_ref = path.as_ref();
    let display = path_ref.display().to_string();
    let file = File::open(path_ref).map_err(|e| ConfigError::Open {
        path: display.clone(),
        source: e,
    })?;
    let reader = BufReader::new(file);
    let mut config = Config::default();
    let mut numbered: [Option<String>; MAX_DEVICES] = Default::default();
    let mut legacy_device: Option<String> = None;
    let mut line_num = 0usize;

    for line in reader.lines() {
        line_num += 1;
        let line = line.map_err(|e| ConfigError::Read {
            path: display.clone(),
            source: e,
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key_raw, value_raw)) = trimmed.split_once('=') else {
            warn!("config:{line_num}: malformed line (no '=')");
            continue;
        };
        let key = key_raw.trim();
        let value = strip_quotes(value_raw.trim());

        if key == "DEVICE" {
            legacy_device = Some(value.to_string());
            continue;
        }
        if let Some(idx) = parse_device_index(key) {
            numbered[idx] = Some(value.to_string());
            continue;
        }

        match key {
            "TEMPERATURE" | "TEMP" | "COLOR_TEMP" | "DAY_TEMP" | "NIGHT_TEMP" => {
                set_int_u32(&mut config.temperature, key, value, 1000, 10000);
            }
            "BRIGHTNESS" => {
                set_float(&mut config.brightness, key, value, 0.1, 1.0);
            }
            "AUTO_ACTIVATE" => config.auto_activate = parse_bool(value),
            "MONITOR_TTY" => set_int_i32(&mut config.monitor_tty, key, value, 1, 12),
            "CHECK_INTERVAL" => set_int_u32(&mut config.check_interval, key, value, 1, 3600),
            "VERBOSE" => config.verbose = parse_bool(value),
            "CONNECTOR" => config.connector = value.to_string(),
            "GAMMA_SIZE" => {
                if value == "0" {
                    config.gamma_size = 0;
                } else {
                    set_int_u32(
                        &mut config.gamma_size,
                        key,
                        value,
                        GAMMA_SIZE_MIN,
                        GAMMA_SIZE_MAX,
                    );
                }
            }
            "BACKEND" => config.backend = value.to_string(),
            _ => {}
        }
    }

    let mut devices: Vec<String> = numbered.into_iter().flatten().collect();
    if devices.is_empty() {
        if let Some(d) = legacy_device {
            devices.push(d);
        }
    }
    if devices.is_empty() {
        let found = device::find_all_devices(MAX_DEVICES);
        if !found.is_empty() {
            info!("Auto-detected {} DRM device(s)", found.len());
            devices = found;
        } else if let Some(d) = device::find_device() {
            devices.push(d);
        }
    }
    if devices.is_empty() {
        devices.push("/dev/dri/card0".to_string());
    }
    config.devices = devices;

    if config.verbose {
        for (i, d) in config.devices.iter().enumerate() {
            info!("Device[{i}]: {d}");
        }
        info!(
            "Target Temperature: {}K, Brightness: {:.2}",
            config.temperature, config.brightness
        );
    }

    Ok(config)
}

fn strip_quotes(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn parse_device_index(key: &str) -> Option<usize> {
    let suffix = key.strip_prefix("DEVICE")?;
    let n: usize = suffix.parse().ok()?;
    if (1..=MAX_DEVICES).contains(&n) {
        Some(n - 1)
    } else {
        None
    }
}

fn set_int_u32(slot: &mut u32, key: &str, value: &str, min: u32, max: u32) {
    match value.parse::<i64>() {
        Ok(v) if v >= min as i64 && v <= max as i64 => *slot = v as u32,
        Ok(v) => warn!(
            "config: {key}={v} out of range [{min},{max}], keeping {}",
            *slot
        ),
        Err(_) => warn!(
            "config: invalid integer for {key}: '{value}', keeping {}",
            *slot
        ),
    }
}

fn set_int_i32(slot: &mut i32, key: &str, value: &str, min: i32, max: i32) {
    match value.parse::<i32>() {
        Ok(v) if v >= min && v <= max => *slot = v,
        Ok(v) => warn!(
            "config: {key}={v} out of range [{min},{max}], keeping {}",
            *slot
        ),
        Err(_) => warn!(
            "config: invalid integer for {key}: '{value}', keeping {}",
            *slot
        ),
    }
}

fn set_float(slot: &mut f64, key: &str, value: &str, min: f64, max: f64) {
    match value.parse::<f64>() {
        Ok(v) if v >= min && v <= max => *slot = v,
        Ok(v) => warn!(
            "config: {key}={v} out of range [{min},{max}], keeping {}",
            *slot
        ),
        Err(_) => warn!(
            "config: invalid float for {key}: '{value}', keeping {}",
            *slot
        ),
    }
}

fn parse_bool(value: &str) -> bool {
    matches!(
        value.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp_config(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_default_config() {
        let c = Config::default();
        assert_eq!(c.temperature, 8000);
        assert_eq!(c.brightness, 1.0);
        assert!(c.auto_activate);
    }

    #[test]
    fn test_load_simple_config() {
        let f = write_temp_config("TEMPERATURE=7500\nBRIGHTNESS=0.9\n");
        let c = load_config(f.path()).unwrap();
        assert_eq!(c.temperature, 7500);
        assert!((c.brightness - 0.9).abs() < 1e-6);
    }

    #[test]
    fn test_load_legacy_day_temp() {
        let f = write_temp_config("DAY_TEMP=8200\n");
        let c = load_config(f.path()).unwrap();
        assert_eq!(c.temperature, 8200);
    }

    #[test]
    fn test_load_device_config_ordered() {
        let f = write_temp_config("DEVICE3=/dev/dri/card2\nDEVICE1=/dev/dri/card0\n");
        let c = load_config(f.path()).unwrap();
        assert_eq!(c.devices, vec!["/dev/dri/card0", "/dev/dri/card2"]);
    }

    #[test]
    fn test_load_backend_config() {
        let f = write_temp_config("BACKEND=gnome\n");
        let c = load_config(f.path()).unwrap();
        assert_eq!(c.backend, "gnome");
    }
}
