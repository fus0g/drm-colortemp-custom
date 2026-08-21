//! Display color calibration backends.
//!
//! Provides two backends:
//! 1. `gnome`: Uses GNOME/colord color management infrastructure with ICC VCGT profiles.
//! 2. `drm`: Uses direct DRM CRTC ioctls (for COSMIC Desktop, TTYs, or root services).

pub mod drm;
pub mod gnome;

use log::info;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Auto,
    Drm,
    Gnome,
}

impl BackendKind {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(BackendKind::Auto),
            "drm" => Some(BackendKind::Drm),
            "gnome" => Some(BackendKind::Gnome),
            _ => None,
        }
    }
}

impl fmt::Display for BackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendKind::Auto => write!(f, "auto"),
            BackendKind::Drm => write!(f, "drm"),
            BackendKind::Gnome => write!(f, "gnome"),
        }
    }
}

/// Detect the active desktop environment and select the most appropriate backend.
pub fn detect_backend() -> BackendKind {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let session_desktop = std::env::var("XDG_SESSION_DESKTOP")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let desktop_session = std::env::var("DESKTOP_SESSION")
        .unwrap_or_default()
        .to_ascii_lowercase();

    let is_gnome = desktop.contains("gnome")
        || session_desktop.contains("gnome")
        || desktop_session.contains("gnome");

    if is_gnome && gnome::is_available() {
        BackendKind::Gnome
    } else {
        BackendKind::Drm
    }
}

/// Resolve `BackendKind::Auto` into a concrete `BackendKind::Drm` or `BackendKind::Gnome`.
pub fn resolve_backend(kind: BackendKind) -> BackendKind {
    match kind {
        BackendKind::Auto => {
            let detected = detect_backend();
            info!("Auto-detected environment backend: {detected}");
            detected
        }
        concrete => concrete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_kind_parse() {
        assert_eq!(BackendKind::parse("auto"), Some(BackendKind::Auto));
        assert_eq!(BackendKind::parse("drm"), Some(BackendKind::Drm));
        assert_eq!(BackendKind::parse("DRM"), Some(BackendKind::Drm));
        assert_eq!(BackendKind::parse("gnome"), Some(BackendKind::Gnome));
        assert_eq!(BackendKind::parse("GNOME"), Some(BackendKind::Gnome));
        assert_eq!(BackendKind::parse("invalid"), None);
    }
}
