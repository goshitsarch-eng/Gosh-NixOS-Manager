//! Unprivileged UI preferences. JSON is canonical; cosmic-config is a best-effort mirror.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::APP_ID;

/// Color scheme preference (GTK parity: System / Light / Dark).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorSchemePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl ColorSchemePreference {
    /// Map to a libcosmic [`cosmic::Theme`].
    pub fn to_theme(self) -> cosmic::Theme {
        match self {
            Self::System => cosmic::theme::system_preference(),
            Self::Light => cosmic::Theme::light(),
            Self::Dark => cosmic::Theme::dark(),
        }
    }
}

/// User preferences stored in XDG config (not privileged `state.json`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserPreferences {
    pub color_scheme: ColorSchemePreference,
}

impl UserPreferences {
    /// Canonical JSON path: `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json`.
    #[must_use]
    pub fn json_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nixos-toolkit")
            .join("preferences.json")
    }

    /// Load order: `override_path` → JSON canonical → cosmic-config → default System.
    #[must_use]
    pub fn load(override_path: Option<&Path>) -> Self {
        if let Some(path) = override_path {
            if let Some(prefs) = Self::load_json(path) {
                return prefs;
            }
        }

        let json_path = Self::json_path();
        if let Some(prefs) = Self::load_json(&json_path) {
            return prefs;
        }

        if let Some(prefs) = Self::load_cosmic() {
            return prefs;
        }

        Self::default()
    }

    fn load_json(path: &Path) -> Option<Self> {
        let data = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    fn load_cosmic() -> Option<Self> {
        let helper = cosmic::cosmic_config::Config::new(APP_ID, 1).ok()?;
        match cosmic::cosmic_config::ConfigGet::get::<ColorSchemePreference>(
            &helper,
            "color_scheme",
        ) {
            Ok(color_scheme) => Some(Self { color_scheme }),
            Err(err) => {
                tracing::debug!(?err, "no cosmic-config color_scheme");
                None
            }
        }
    }

    /// Write JSON (canonical) and best-effort cosmic-config mirror.
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::json_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, json)?;

        // TODO: keep cosmic-config in sync when the CosmicConfigEntry derive is wired.
        // JSON remains the portable source of truth (DECISIONS C5).
        if let Ok(helper) = cosmic::cosmic_config::Config::new(APP_ID, 1) {
            if let Err(err) =
                cosmic::cosmic_config::ConfigSet::set(&helper, "color_scheme", self.color_scheme)
            {
                tracing::debug!(?err, "failed to mirror prefs to cosmic-config");
            }
        }

        Ok(())
    }

    /// Save to an explicit path (tests).
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_round_trip() {
        let dir =
            std::env::temp_dir().join(format!("nixos-toolkit-prefs-test-{}", std::process::id()));
        let path = dir.join("preferences.json");
        let prefs = UserPreferences {
            color_scheme: ColorSchemePreference::Dark,
        };
        prefs.save_to(&path).expect("write prefs");
        let loaded = UserPreferences::load(Some(&path));
        assert_eq!(loaded.color_scheme, ColorSchemePreference::Dark);
        let _ = std::fs::remove_dir_all(dir);
    }
}
