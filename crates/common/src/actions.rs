//! Action registry and definitions for NixOS Toolkit
//!
//! This module defines the extensible action system that allows adding
//! new profiles, bundles, and system actions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuArch {
    X86_64,
    Aarch64,
    Unknown,
}

impl CpuArch {
    /// Detect current CPU architecture
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        return CpuArch::X86_64;

        #[cfg(target_arch = "aarch64")]
        return CpuArch::Aarch64;

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        return CpuArch::Unknown;
    }

    pub fn is_arm(&self) -> bool {
        matches!(self, CpuArch::Aarch64)
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "ARM64 (aarch64)",
            Self::Unknown => "Unknown",
        }
    }
}

/// ARM compatibility level for bundles/profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmCompat {
    /// Fully compatible with ARM
    Full,
    /// Mostly works, some packages unavailable
    Partial,
    /// Very limited ARM support
    Limited,
    /// Not available on ARM at all
    None,
}

impl ArmCompat {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Full => "Full ARM support",
            Self::Partial => "Partial ARM support",
            Self::Limited => "Limited ARM support",
            Self::None => "x86_64 only",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Full => "emblem-ok-symbolic",
            Self::Partial => "dialog-warning-symbolic",
            Self::Limited => "dialog-warning-symbolic",
            Self::None => "action-unavailable-symbolic",
        }
    }
}

/// Unique identifier for an action
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionId(pub String);

impl ActionId {
    pub fn new(category: &str, name: &str) -> Self {
        Self(format!("{}.{}", category, name))
    }

    pub fn from_string(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for ActionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Categories for organizing actions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionCategory {
    Desktop,
    Development,
    Gaming,
    Multimedia,
    Virtualization,
    System,
    Network,
    Security,
    Office,
}

impl ActionCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Desktop => "Desktop Environments",
            Self::Development => "Development Tools",
            Self::Gaming => "Gaming",
            Self::Multimedia => "Multimedia",
            Self::Virtualization => "Virtualization",
            Self::System => "System Settings",
            Self::Network => "Networking",
            Self::Security => "Security",
            Self::Office => "Office & Productivity",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            Self::Desktop => "user-desktop-symbolic",
            Self::Development => "applications-engineering-symbolic",
            Self::Gaming => "applications-games-symbolic",
            Self::Multimedia => "applications-multimedia-symbolic",
            Self::Virtualization => "computer-symbolic",
            Self::System => "preferences-system-symbolic",
            Self::Network => "network-wired-symbolic",
            Self::Security => "security-high-symbolic",
            Self::Office => "x-office-document-symbolic",
        }
    }
}

/// Configuration value types for action settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    Bool(bool),
    String(String),
    Integer(i64),
    Float(f64),
    List(Vec<ConfigValue>),
}

/// User-provided configuration for an action
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionConfig {
    pub enabled: bool,
    pub values: HashMap<String, ConfigValue>,
}

impl ActionConfig {
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            values: HashMap::new(),
        }
    }

    pub fn with_value(mut self, key: impl Into<String>, value: ConfigValue) -> Self {
        self.values.insert(key.into(), value);
        self
    }
}

/// Metadata describing an action for the UI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMetadata {
    pub id: ActionId,
    pub name: String,
    pub description: String,
    pub category: ActionCategory,
    pub icon: Option<String>,
    pub template_path: Option<String>,
    pub conflicts: Vec<ActionId>,
    pub requires: Vec<ActionId>,
}

/// Definition of a desktop environment profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub template: String,
    pub display_manager: String,
    /// ARM compatibility level
    pub arm_compat: ArmCompat,
    /// Note about ARM compatibility (if any)
    pub arm_note: Option<String>,
}

/// Definition of a package within a bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageDef {
    /// Nixpkgs package name (e.g., "vscode")
    pub id: String,
    /// Human-readable display name (e.g., "Visual Studio Code")
    pub display_name: String,
}

impl PackageDef {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
        }
    }
}

/// Definition of a software bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub category: ActionCategory,
    pub template: String,
    pub packages: Vec<PackageDef>,
    /// ARM compatibility level
    pub arm_compat: ArmCompat,
    /// Note about what's unavailable on ARM (if any)
    pub arm_note: Option<String>,
}

/// Get all available profile definitions
pub fn default_profiles() -> Vec<ProfileDef> {
    vec![
        ProfileDef {
            id: "gnome".into(),
            name: "GNOME".into(),
            description:
                "Modern, elegant desktop environment with a focus on simplicity and productivity"
                    .into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/gnome.nix".into(),
            display_manager: "gdm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "kde".into(),
            name: "KDE Plasma".into(),
            description: "Feature-rich, highly customizable desktop environment".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/kde.nix".into(),
            display_manager: "sddm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "xfce".into(),
            name: "XFCE".into(),
            description:
                "Lightweight, fast desktop environment with traditional desktop experience".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/xfce.nix".into(),
            display_manager: "lightdm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "mate".into(),
            name: "MATE".into(),
            description: "Traditional desktop based on GNOME 2, intuitive and lightweight".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/mate.nix".into(),
            display_manager: "lightdm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "cinnamon".into(),
            name: "Cinnamon".into(),
            description: "Modern desktop from Linux Mint with traditional layout".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/cinnamon.nix".into(),
            display_manager: "lightdm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "pantheon".into(),
            name: "Pantheon".into(),
            description: "Beautiful, clean desktop from elementary OS".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/pantheon.nix".into(),
            display_manager: "lightdm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "cosmic".into(),
            name: "COSMIC".into(),
            description: "System76's modern, Rust-based Wayland desktop (stable 1.0)".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/cosmic.nix".into(),
            display_manager: "cosmic-greeter".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        // Tiling Window Managers
        ProfileDef {
            id: "hyprland".into(),
            name: "Hyprland".into(),
            description: "Dynamic tiling Wayland compositor with smooth animations".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/hyprland.nix".into(),
            display_manager: "sddm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "sway".into(),
            name: "Sway".into(),
            description: "i3-compatible tiling Wayland compositor".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/sway.nix".into(),
            display_manager: "sddm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "i3".into(),
            name: "i3".into(),
            description: "Popular tiling window manager for X11, highly configurable".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/i3.nix".into(),
            display_manager: "lightdm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        // Additional DEs
        ProfileDef {
            id: "budgie".into(),
            name: "Budgie".into(),
            description: "Modern desktop focusing on simplicity and elegance".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/budgie.nix".into(),
            display_manager: "lightdm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "lxqt".into(),
            name: "LXQt".into(),
            description: "Lightweight Qt-based desktop environment".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/lxqt.nix".into(),
            display_manager: "sddm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        ProfileDef {
            id: "enlightenment".into(),
            name: "Enlightenment".into(),
            description: "Unique, visually stunning desktop with compositing effects".into(),
            icon: "desktop-symbolic".into(),
            template: "profiles/enlightenment.nix".into(),
            display_manager: "lightdm".into(),
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
    ]
}

/// Get all available bundle definitions
pub fn default_bundles() -> Vec<BundleDef> {
    vec![
        // Development
        BundleDef {
            id: "devtools".into(),
            name: "Development Tools".into(),
            description: "Essential development tools: Git, editors, compilers, and containers"
                .into(),
            icon: "applications-engineering-symbolic".into(),
            category: ActionCategory::Development,
            template: "bundles/devtools.nix".into(),
            packages: vec![
                PackageDef::new("git", "Git"),
                PackageDef::new("neovim", "Neovim"),
                PackageDef::new("emacs", "Emacs"),
                PackageDef::new("vscode", "Visual Studio Code"),
                PackageDef::new("vscodium", "VSCodium"),
                PackageDef::new("zed-editor", "Zed Editor"),
                PackageDef::new("rustup", "Rustup"),
                PackageDef::new("nodejs", "Node.js"),
                PackageDef::new("python3", "Python 3"),
                PackageDef::new("docker", "Docker"),
            ],
            arm_compat: ArmCompat::Partial,
            arm_note: Some("VSCode binary not available on ARM".into()),
        },
        // AI Tools
        BundleDef {
            id: "ai-tools".into(),
            name: "AI Tools".into(),
            description: "Local LLMs and AI development tools".into(),
            icon: "applications-science-symbolic".into(),
            category: ActionCategory::Development,
            template: "bundles/ai-tools.nix".into(),
            packages: vec![PackageDef::new("ollama", "Ollama")],
            arm_compat: ArmCompat::Partial,
            arm_note: Some("Some AI tools may have limited ARM support".into()),
        },
        // Gaming
        BundleDef {
            id: "gaming".into(),
            name: "Gaming".into(),
            description: "Steam, Lutris, and gaming utilities for Linux gaming".into(),
            icon: "applications-games-symbolic".into(),
            category: ActionCategory::Gaming,
            template: "bundles/gaming.nix".into(),
            packages: vec![
                PackageDef::new("steam", "Steam"),
                PackageDef::new("lutris", "Lutris"),
                PackageDef::new("heroic", "Heroic Games Launcher"),
                PackageDef::new("bottles", "Bottles"),
                PackageDef::new("mangohud", "MangoHud"),
                PackageDef::new("gamemode", "GameMode"),
            ],
            arm_compat: ArmCompat::None,
            arm_note: Some("Steam, Lutris, Heroic, Bottles, Wine are x86_64 only".into()),
        },
        // Virtualization
        BundleDef {
            id: "virtualization".into(),
            name: "KVM/QEMU Virtualization".into(),
            description: "Hardware-accelerated VMs with libvirt and virt-manager".into(),
            icon: "computer-symbolic".into(),
            category: ActionCategory::Virtualization,
            template: "bundles/virtualization.nix".into(),
            packages: vec![
                PackageDef::new("virt-manager", "Virtual Machine Manager"),
                PackageDef::new("qemu", "QEMU"),
                PackageDef::new("spice-gtk", "SPICE GTK"),
            ],
            arm_compat: ArmCompat::Limited,
            arm_note: Some("OVMF/UEFI firmware is x86_64 only; use AAVMF for ARM VMs".into()),
        },
        BundleDef {
            id: "virtualbox".into(),
            name: "VirtualBox".into(),
            description: "Oracle VirtualBox with Extension Pack (non-free)".into(),
            icon: "computer-symbolic".into(),
            category: ActionCategory::Virtualization,
            template: "bundles/virtualbox.nix".into(),
            packages: vec![PackageDef::new("virtualbox", "VirtualBox")],
            arm_compat: ArmCompat::None,
            arm_note: Some("VirtualBox is x86_64 only".into()),
        },
        BundleDef {
            id: "containers".into(),
            name: "Container Runtime".into(),
            description: "Podman/Docker container support with compose".into(),
            icon: "package-x-generic-symbolic".into(),
            category: ActionCategory::Virtualization,
            template: "bundles/containers.nix".into(),
            packages: vec![
                PackageDef::new("podman", "Podman"),
                PackageDef::new("docker-compose", "Docker Compose"),
                PackageDef::new("buildah", "Buildah"),
                PackageDef::new("skopeo", "Skopeo"),
            ],
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        // System
        BundleDef {
            id: "flatpak".into(),
            name: "Flatpak Support".into(),
            description: "Enable Flatpak for sandboxed application installation".into(),
            icon: "package-x-generic-symbolic".into(),
            category: ActionCategory::System,
            template: "bundles/flatpak.nix".into(),
            packages: vec![PackageDef::new("flatpak", "Flatpak")],
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        // Multimedia
        BundleDef {
            id: "multimedia".into(),
            name: "Multimedia".into(),
            description: "Audio, video, and graphics applications with PipeWire".into(),
            icon: "applications-multimedia-symbolic".into(),
            category: ActionCategory::Multimedia,
            template: "bundles/multimedia.nix".into(),
            packages: vec![
                PackageDef::new("vlc", "VLC Media Player"),
                PackageDef::new("mpv", "mpv"),
                PackageDef::new("gimp", "GIMP"),
                PackageDef::new("inkscape", "Inkscape"),
                PackageDef::new("obs-studio", "OBS Studio"),
                PackageDef::new("audacity", "Audacity"),
            ],
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        // Office
        BundleDef {
            id: "office".into(),
            name: "Office & Productivity".into(),
            description: "LibreOffice, PDF tools, email, and productivity apps".into(),
            icon: "x-office-document-symbolic".into(),
            category: ActionCategory::Office,
            template: "bundles/office.nix".into(),
            packages: vec![
                PackageDef::new("libreoffice", "LibreOffice"),
                PackageDef::new("onlyoffice-desktopeditors", "OnlyOffice"),
                PackageDef::new("thunderbird", "Thunderbird"),
                PackageDef::new("evince", "Evince"),
                PackageDef::new("obsidian", "Obsidian"),
            ],
            arm_compat: ArmCompat::Partial,
            arm_note: Some("Obsidian and OnlyOffice binaries not available on ARM".into()),
        },
        // Security Tools
        BundleDef {
            id: "security".into(),
            name: "Security Tools".into(),
            description: "Password managers, encryption, and security utilities".into(),
            icon: "security-high-symbolic".into(),
            category: ActionCategory::Security,
            template: "bundles/security.nix".into(),
            packages: vec![
                PackageDef::new("keepassxc", "KeePassXC"),
                PackageDef::new("bitwarden", "Bitwarden"),
                PackageDef::new("_1password-gui", "1Password"),
                PackageDef::new("veracrypt", "VeraCrypt"),
                PackageDef::new("gnupg", "GnuPG"),
                PackageDef::new("age", "age"),
            ],
            arm_compat: ArmCompat::Partial,
            arm_note: Some("1Password binary not available on ARM".into()),
        },
        // Communication
        BundleDef {
            id: "communication".into(),
            name: "Communication".into(),
            description: "Chat, video calls, and messaging applications".into(),
            icon: "user-available-symbolic".into(),
            category: ActionCategory::Network,
            template: "bundles/communication.nix".into(),
            packages: vec![
                PackageDef::new("discord", "Discord"),
                PackageDef::new("signal-desktop", "Signal"),
                PackageDef::new("element-desktop", "Element"),
                PackageDef::new("slack", "Slack"),
                PackageDef::new("zoom-us", "Zoom"),
            ],
            arm_compat: ArmCompat::Partial,
            arm_note: Some("Slack and Zoom binaries are x86_64 only".into()),
        },
        // Browsers
        BundleDef {
            id: "browsers".into(),
            name: "Web Browsers".into(),
            description: "Additional web browsers with different privacy/feature focuses".into(),
            icon: "web-browser-symbolic".into(),
            category: ActionCategory::Network,
            template: "bundles/browsers.nix".into(),
            packages: vec![
                PackageDef::new("firefox", "Firefox"),
                PackageDef::new("chromium", "Chromium"),
                PackageDef::new("google-chrome", "Google Chrome"),
                PackageDef::new("brave", "Brave"),
                PackageDef::new("tor-browser-bundle-bin", "Tor Browser"),
            ],
            arm_compat: ArmCompat::Partial,
            arm_note: Some("Google Chrome binary not available on ARM".into()),
        },
        // Science & Math
        BundleDef {
            id: "science".into(),
            name: "Science & Math".into(),
            description: "Scientific computing, math tools, and LaTeX".into(),
            icon: "accessories-calculator-symbolic".into(),
            category: ActionCategory::Development,
            template: "bundles/science.nix".into(),
            packages: vec![
                PackageDef::new("octave", "GNU Octave"),
                PackageDef::new("julia", "Julia"),
                PackageDef::new("gnuplot", "Gnuplot"),
            ],
            arm_compat: ArmCompat::Partial,
            arm_note: Some("RStudio binary not available; Julia has limited ARM support".into()),
        },
        // 3D & CAD
        BundleDef {
            id: "cad".into(),
            name: "3D & CAD".into(),
            description: "3D modeling, CAD, and design software".into(),
            icon: "applications-graphics-symbolic".into(),
            category: ActionCategory::Multimedia,
            template: "bundles/cad.nix".into(),
            packages: vec![
                PackageDef::new("blender", "Blender"),
                PackageDef::new("freecad", "FreeCAD"),
                PackageDef::new("openscad", "OpenSCAD"),
                PackageDef::new("kicad", "KiCad"),
            ],
            arm_compat: ArmCompat::Limited,
            arm_note: Some("Blender and FreeCAD have limited ARM support".into()),
        },
        // System Utilities
        BundleDef {
            id: "utilities".into(),
            name: "System Utilities".into(),
            description: "Helpful command-line and system tools".into(),
            icon: "utilities-system-monitor-symbolic".into(),
            category: ActionCategory::System,
            template: "bundles/utilities.nix".into(),
            packages: vec![
                PackageDef::new("htop", "htop"),
                PackageDef::new("btop", "btop"),
                PackageDef::new("fastfetch", "Fastfetch"),
                PackageDef::new("tmux", "tmux"),
                PackageDef::new("tree", "tree"),
                PackageDef::new("unzip", "unzip"),
                PackageDef::new("wget", "wget"),
                PackageDef::new("curl", "curl"),
                PackageDef::new("appimage-run", "AppImage Run"),
            ],
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
        // Fonts
        BundleDef {
            id: "fonts".into(),
            name: "Fonts Collection".into(),
            description: "Popular fonts for development, documents, and design".into(),
            icon: "font-x-generic-symbolic".into(),
            category: ActionCategory::System,
            template: "bundles/fonts.nix".into(),
            packages: vec![
                PackageDef::new("nerd-fonts.fira-code", "Fira Code Nerd Font"),
                PackageDef::new("nerd-fonts.jetbrains-mono", "JetBrains Mono Nerd Font"),
                PackageDef::new("fira-code", "Fira Code"),
                PackageDef::new("jetbrains-mono", "JetBrains Mono"),
                PackageDef::new("inter", "Inter"),
                PackageDef::new("noto-fonts", "Noto Fonts"),
            ],
            arm_compat: ArmCompat::Full,
            arm_note: None,
        },
    ]
}

/// System action definition (hostname, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemActionDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub action_type: SystemActionType,
}

/// Type of system action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemActionType {
    /// Text input (hostname, DNS IP)
    TextInput { placeholder: String },
    /// Toggle on/off
    Toggle,
    /// Selection from options
    Select { options: Vec<String> },
    /// User selection for group membership
    UserGroup { group: String },
}

/// Get available system actions
pub fn default_system_actions() -> Vec<SystemActionDef> {
    vec![
        SystemActionDef {
            id: "hostname".into(),
            name: "Hostname".into(),
            description: "Change the system hostname".into(),
            icon: "computer-symbolic".into(),
            action_type: SystemActionType::TextInput {
                placeholder: "nixos".into(),
            },
        },
        SystemActionDef {
            id: "dns".into(),
            name: "Custom DNS".into(),
            description: "Set a custom DNS resolver (e.g., 1.1.1.1, 8.8.8.8)".into(),
            icon: "network-server-symbolic".into(),
            action_type: SystemActionType::TextInput {
                placeholder: "1.1.1.1".into(),
            },
        },
        SystemActionDef {
            id: "libvirtd_user".into(),
            name: "libvirtd Group".into(),
            description: "Add user to libvirtd group for KVM access".into(),
            icon: "system-users-symbolic".into(),
            action_type: SystemActionType::UserGroup {
                group: "libvirtd".into(),
            },
        },
        SystemActionDef {
            id: "docker_user".into(),
            name: "Docker Group".into(),
            description: "Add user to docker group for rootless Docker".into(),
            icon: "system-users-symbolic".into(),
            action_type: SystemActionType::UserGroup {
                group: "docker".into(),
            },
        },
        SystemActionDef {
            id: "vboxusers".into(),
            name: "VirtualBox Group".into(),
            description: "Add user to vboxusers group for VirtualBox access".into(),
            icon: "system-users-symbolic".into(),
            action_type: SystemActionType::UserGroup {
                group: "vboxusers".into(),
            },
        },
    ]
}

/// Maintenance action definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintenanceActionDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub warning: Option<String>,
    pub command: String,
}

/// Get available maintenance actions
pub fn default_maintenance_actions() -> Vec<MaintenanceActionDef> {
    vec![
        MaintenanceActionDef {
            id: "gc_unreachable".into(),
            name: "Garbage Collect".into(),
            description: "Delete unreachable store objects to free disk space".into(),
            icon: "user-trash-symbolic".into(),
            warning: None,
            command: "nix-collect-garbage".into(),
        },
        MaintenanceActionDef {
            id: "gc_all".into(),
            name: "Delete Old Generations".into(),
            description: "Delete all old system generations (keeps current only)".into(),
            icon: "user-trash-full-symbolic".into(),
            warning: Some("This will delete all old configurations. You won't be able to roll back to previous generations!".into()),
            command: "nix-collect-garbage -d".into(),
        },
        MaintenanceActionDef {
            id: "optimize_store".into(),
            name: "Optimize Store".into(),
            description: "Deduplicate files in the Nix store to save space".into(),
            icon: "drive-harddisk-symbolic".into(),
            warning: None,
            command: "nix-store --optimise".into(),
        },
        MaintenanceActionDef {
            id: "verify_store".into(),
            name: "Verify Store".into(),
            description: "Check Nix store for integrity issues".into(),
            icon: "security-high-symbolic".into(),
            warning: None,
            command: "nix-store --verify --check-contents".into(),
        },
        MaintenanceActionDef {
            id: "update_channels".into(),
            name: "Update Channels".into(),
            description: "Update Nix channels to latest versions".into(),
            icon: "software-update-available-symbolic".into(),
            warning: None,
            command: "nix-channel --update".into(),
        },
    ]
}
