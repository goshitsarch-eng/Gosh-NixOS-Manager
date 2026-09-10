//! NixOS Toolkit GUI library (libcosmic). Public API is for tests.

pub mod app;
pub mod config;
pub mod core;
pub mod helper;
pub mod i18n;
pub mod integration;
pub mod message;
pub mod state;
pub mod view;
pub mod widget;

pub use app::{AppModel, Flags};
pub use config::{ColorSchemePreference, UserPreferences};
pub use helper::spawn::SpawnSpec;
pub use message::{Dialog, HelperEvent, HelperOp, Intent, Message, Page, RollbackMode};
pub use state::AppState;

/// Application ID (D-Bus / Flatpak / cosmic-config).
pub const APP_ID: &str = "io.github.goshitsarch_eng.NixosToolkit";
