//! Common types and traits for NixOS Toolkit
//!
//! This crate provides shared functionality between the GUI and helper binaries:
//! - Action registry and definitions
//! - IPC message types
//! - System configuration types
//! - Nix code generation traits

pub mod actions;
pub mod config;
pub mod ipc;
pub mod nix;

pub use actions::{
    ActionCategory, ActionConfig, ActionId, ActionMetadata, ArmCompat, BundleDef, CpuArch,
    MaintenanceActionDef, ProfileDef, SystemActionDef, SystemActionType,
};
pub use config::{ConfigMode, IntegrationStatus, SystemInfo};
pub use ipc::{HelperRequest, HelperResponse, LogLevel, RebuildType};
pub use nix::{NixGenError, NixGenOptions, NixOutput};
