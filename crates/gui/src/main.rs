//! NixOS Toolkit - Main Entry Point
//!
//! A GTK4/libadwaita application for declarative NixOS system management.

mod app;
mod helper;
mod integration;
mod pages;
mod preferences;
mod state;
mod window;

use gtk::prelude::*;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Application ID
pub const APP_ID: &str = "org.nixos-toolkit.app";

fn main() -> glib::ExitCode {
    // Initialize logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive("nixos_toolkit=info".parse().unwrap()))
        .init();

    tracing::info!("Starting NixOS Toolkit");

    // Create and run the application
    let app = app::NixosToolkitApp::new();
    app.run()
}
