//! User preferences (theme, UI settings) - stored locally without privilege

use gtk::glib;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Color scheme preference
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorSchemePreference {
    /// Follow system/desktop preference
    #[default]
    System,
    /// Always use light mode
    Light,
    /// Always use dark mode
    Dark,
}

impl ColorSchemePreference {
    pub fn to_adw_color_scheme(self) -> adw::ColorScheme {
        match self {
            Self::System => adw::ColorScheme::Default,
            Self::Light => adw::ColorScheme::ForceLight,
            Self::Dark => adw::ColorScheme::ForceDark,
        }
    }
}

/// User preferences (stored in XDG config, NOT privileged state)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Theme/color scheme preference
    pub color_scheme: ColorSchemePreference,
}

impl UserPreferences {
    /// Get the preferences file path
    pub fn path() -> PathBuf {
        glib::user_config_dir().join("nixos-toolkit").join("preferences.json")
    }

    /// Load preferences from disk (synchronous, fast, no privileges)
    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Save preferences to disk (creates directory if needed)
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}
