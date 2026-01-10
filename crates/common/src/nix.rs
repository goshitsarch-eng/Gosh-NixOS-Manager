//! Nix code generation utilities

use crate::actions::{BundleDef, ProfileDef};
use crate::config::paths;
use crate::ipc::{NetworkConfig, ServicesConfig};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during Nix generation
#[derive(Debug, Error)]
pub enum NixGenError {
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Failed to read template: {0}")]
    ReadError(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Output from Nix generation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NixOutput {
    /// The generated Nix code
    pub content: String,
    /// Path where this should be written
    pub path: String,
}

/// Configuration options for Nix generation
#[derive(Debug, Clone, Default)]
pub struct NixGenOptions<'a> {
    pub profile: Option<&'a ProfileDef>,
    pub bundles: Vec<&'a BundleDef>,
    /// Per-bundle package selections: bundle_id -> list of enabled package names.
    /// If a bundle is not in this map, all its packages are included.
    pub bundle_packages: std::collections::HashMap<String, Vec<String>>,
    pub hostname: Option<&'a str>,
    pub dns_servers: Vec<String>,
    pub user_groups: Vec<String>,
    pub username: Option<&'a str>,
    pub bluetooth_enabled: bool,
    pub custom_packages: Vec<String>,
    pub network_config: NetworkConfig,
    pub services_config: ServicesConfig,
}

impl<'a> NixGenOptions<'a> {
    /// Get the packages that should be enabled for a specific bundle.
    /// If bundle_packages contains an entry for this bundle, returns that list.
    /// Otherwise, returns all packages from the bundle definition.
    pub fn get_bundle_packages(&self, bundle: &BundleDef) -> Vec<String> {
        if let Some(packages) = self.bundle_packages.get(&bundle.id) {
            packages.clone()
        } else {
            bundle.packages.clone()
        }
    }
}

/// Generate the selected.nix file content with full options
pub fn generate_selected_nix_full(options: &NixGenOptions) -> String {
    let mut imports = Vec::new();

    // Add profile import
    if let Some(p) = options.profile {
        imports.push(format!("    ../profiles/{}.nix", p.id));
    }

    // Add bundle imports
    for bundle in &options.bundles {
        imports.push(format!("    ../bundles/{}.nix", bundle.id));
    }

    // Add hostname config if set
    if options.hostname.is_some() {
        imports.push("    ./hostname.nix".to_string());
    }

    // Add DNS config if servers are set
    if !options.dns_servers.is_empty() {
        imports.push("    ./dns.nix".to_string());
    }

    // Add user groups config if set
    if !options.user_groups.is_empty() && options.username.is_some() {
        imports.push("    ./users.nix".to_string());
    }

    // Add custom packages config if set
    if !options.custom_packages.is_empty() {
        imports.push("    ./custom-packages.nix".to_string());
    }

    // Add hardware config if Bluetooth is enabled
    if options.bluetooth_enabled {
        imports.push("    ./hardware.nix".to_string());
    }

    // Add network config if network settings are configured
    if options.network_config.has_settings() {
        imports.push("    ./network.nix".to_string());
    }

    // Add services config if services are configured
    if options.services_config.has_settings() {
        imports.push("    ./services.nix".to_string());
    }

    // Build the imports section
    let imports_str = if imports.is_empty() {
        "    # No profiles or bundles selected".to_string()
    } else {
        imports.join("\n")
    };

    format!(
        r#"# NixOS Toolkit - Managed Configuration
# DO NOT EDIT MANUALLY - Changes will be overwritten
#
# This file is managed by nixos-toolkit. To make changes,
# use the GUI application and click "Apply".
#
# Selected profile: {}
# Enabled bundles: {}
# DNS servers: {}
# User groups: {}
# Bluetooth: {}
# Custom packages: {}
# Network: {}
# Services: {}

{{ config, lib, pkgs, ... }}:

{{
  imports = [
{}
  ];
}}
"#,
        options.profile.map(|p| p.name.as_str()).unwrap_or("None"),
        if options.bundles.is_empty() {
            "None".to_string()
        } else {
            options.bundles
                .iter()
                .map(|b| b.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        },
        if options.dns_servers.is_empty() {
            "None".to_string()
        } else {
            options.dns_servers.join(", ")
        },
        if options.user_groups.is_empty() {
            "None".to_string()
        } else {
            options.user_groups.join(", ")
        },
        if options.bluetooth_enabled { "Enabled" } else { "Disabled" },
        if options.custom_packages.is_empty() {
            "None".to_string()
        } else {
            options.custom_packages.join(", ")
        },
        if options.network_config.has_settings() { "Configured" } else { "Default" },
        if options.services_config.has_settings() {
            options.services_config.enabled_services().join(", ")
        } else {
            "None".to_string()
        },
        imports_str,
    )
}

/// Generate the selected.nix file content (legacy signature for compatibility)
pub fn generate_selected_nix(
    profile: Option<&ProfileDef>,
    bundles: &[&BundleDef],
    hostname: Option<&str>,
) -> String {
    let options = NixGenOptions {
        profile,
        bundles: bundles.to_vec(),
        hostname,
        ..Default::default()
    };
    generate_selected_nix_full(&options)
}

/// Generate the hostname.nix file content
pub fn generate_hostname_nix(hostname: &str) -> String {
    format!(
        r#"# NixOS Toolkit - Hostname Configuration
# DO NOT EDIT MANUALLY
# This setting takes precedence - remove networking.hostName from configuration.nix

{{ config, lib, pkgs, ... }}:

{{
  networking.hostName = lib.mkForce "{}";
}}
"#,
        hostname
    )
}

/// Generate the dns.nix file content for custom DNS configuration
pub fn generate_dns_nix(servers: &[String]) -> String {
    if servers.is_empty() {
        return String::new();
    }

    let servers_str = servers
        .iter()
        .map(|s| format!("    \"{}\"", s))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"# NixOS Toolkit - DNS Configuration
# DO NOT EDIT MANUALLY
# Uses lib.mkDefault so explicit settings in configuration.nix take precedence

{{ config, lib, pkgs, ... }}:

{{
  networking.nameservers = lib.mkDefault [
{}
  ];
}}
"#,
        servers_str
    )
}

/// Generate the users.nix file content for user group membership
pub fn generate_user_groups_nix(username: &str, groups: &[String]) -> String {
    if username.is_empty() || groups.is_empty() {
        return String::new();
    }

    let groups_str = groups
        .iter()
        .map(|g| format!("\"{}\"", g))
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        r#"# NixOS Toolkit - User Groups Configuration
# DO NOT EDIT MANUALLY

{{ config, lib, pkgs, ... }}:

{{
  users.users.{} = {{
    extraGroups = [ {} ];
  }};
}}
"#,
        username, groups_str
    )
}

/// Generate the custom-packages.nix file content for manually added packages
pub fn generate_custom_packages_nix(packages: &[String]) -> String {
    if packages.is_empty() {
        return String::new();
    }

    let packages_str = packages
        .iter()
        .map(|p| format!("    {}", p))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"# NixOS Toolkit - Custom Packages
# DO NOT EDIT MANUALLY - Managed by nixos-toolkit
#
# Custom packages: {}

{{ config, lib, pkgs, ... }}:

{{
  environment.systemPackages = with pkgs; [
{}
  ];
}}
"#,
        packages.join(", "),
        packages_str
    )
}

/// Generate the firewall.nix file content
pub fn generate_firewall_nix(enabled: bool, tcp_ports: &[u16], udp_ports: &[u16]) -> String {
    let tcp_str = tcp_ports
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(" ");

    let udp_str = udp_ports
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        r#"# NixOS Toolkit - Firewall Configuration
# DO NOT EDIT MANUALLY

{{ config, lib, pkgs, ... }}:

{{
  networking.firewall = {{
    enable = {};
    allowedTCPPorts = [ {} ];
    allowedUDPPorts = [ {} ];
  }};
}}
"#,
        if enabled { "true" } else { "false" },
        tcp_str,
        udp_str
    )
}

/// Generate the ssh.nix file content
pub fn generate_ssh_nix(
    enabled: bool,
    port: u16,
    password_auth: bool,
    root_login: &str,
    fail2ban: bool,
) -> String {
    let mut config = format!(
        r#"# NixOS Toolkit - SSH Configuration
# DO NOT EDIT MANUALLY

{{ config, lib, pkgs, ... }}:

{{
  services.openssh = {{
    enable = {};
    ports = [ {} ];
    settings = {{
      PasswordAuthentication = {};
      PermitRootLogin = "{}";
    }};
  }};
"#,
        if enabled { "true" } else { "false" },
        port,
        if password_auth { "true" } else { "false" },
        root_login
    );

    if fail2ban {
        config.push_str(r#"
  services.fail2ban = {
    enable = true;
    jails.sshd = {
      enabled = true;
    };
  };
"#);
    }

    config.push_str("}\n");
    config
}

/// Generate the hardware.nix file content for GPU, audio, bluetooth, power
pub fn generate_hardware_nix(
    nvidia_enabled: bool,
    nvidia_open: bool,
    audio_pipewire: bool,
    bluetooth_enabled: bool,
    tlp_enabled: bool,
    thermald_enabled: bool,
) -> String {
    let mut sections = Vec::new();

    // NVIDIA section
    if nvidia_enabled {
        sections.push(format!(
            r#"  # NVIDIA GPU
  services.xserver.videoDrivers = [ "nvidia" ];
  hardware.nvidia = {{
    modesetting.enable = true;
    powerManagement.enable = true;
    open = {};
  }};
  hardware.graphics.enable = true;"#,
            if nvidia_open { "true" } else { "false" }
        ));
    }

    // Audio section
    if audio_pipewire {
        sections.push(r#"  # PipeWire Audio
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    jack.enable = true;
  };
  security.rtkit.enable = true;"#.to_string());
    }

    // Bluetooth section
    if bluetooth_enabled {
        sections.push(r#"  # Bluetooth
  hardware.bluetooth = {
    enable = true;
    powerOnBoot = true;
  };
  services.blueman.enable = true;"#.to_string());
    }

    // Power management
    if tlp_enabled {
        sections.push(r#"  # TLP Power Management
  services.tlp = {
    enable = true;
    settings = {
      CPU_SCALING_GOVERNOR_ON_AC = "performance";
      CPU_SCALING_GOVERNOR_ON_BAT = "powersave";
    };
  };
  services.power-profiles-daemon.enable = false;"#.to_string());
    }

    if thermald_enabled {
        sections.push(r#"  # Thermal Management
  services.thermald.enable = true;"#.to_string());
    }

    format!(
        r#"# NixOS Toolkit - Hardware Configuration
# DO NOT EDIT MANUALLY

{{ config, lib, pkgs, ... }}:

{{
{}
}}
"#,
        sections.join("\n\n")
    )
}

/// Generate the services.nix file content
pub fn generate_services_nix(services: &[&str]) -> String {
    let mut sections = Vec::new();

    for service in services {
        let section = match *service {
            "printing" => r#"  # Printing
  services.printing.enable = true;
  services.avahi.enable = true;
  services.avahi.nssmdns4 = true;"#,
            "avahi" => r#"  # Avahi/mDNS
  services.avahi = {
    enable = true;
    nssmdns4 = true;
    publish.enable = true;
  };"#,
            "fwupd" => r#"  # Firmware Updates
  services.fwupd.enable = true;"#,
            "upower" => r#"  # UPower
  services.upower.enable = true;"#,
            "networkmanager" => r#"  # NetworkManager
  networking.networkmanager.enable = true;"#,
            "resolved" => r#"  # systemd-resolved
  services.resolved.enable = true;"#,
            "rustdesk" => r#"  # RustDesk Remote Desktop
  services.rustdesk-server.enable = true;
  environment.systemPackages = with pkgs; [ rustdesk ];"#,
            "syncthing" => r#"  # Syncthing
  services.syncthing.enable = true;"#,
            "locate" => r#"  # Locate Database
  services.locate = {
    enable = true;
    package = pkgs.plocate;
    localuser = null;
  };"#,
            "flatpak" => r#"  # Flatpak
  services.flatpak.enable = true;
  xdg.portal.enable = true;"#,
            "gnome_keyring" => r#"  # GNOME Keyring
  services.gnome.gnome-keyring.enable = true;"#,
            "gnome_tweaks" => r#"  # GNOME Tweaks
  environment.systemPackages = with pkgs; [ gnome-tweaks ];"#,
            "dconf" => r#"  # dconf
  programs.dconf.enable = true;"#,
            "docker" => r#"  # Docker
  virtualisation.docker.enable = true;"#,
            "libvirtd" => r#"  # libvirtd
  virtualisation.libvirtd.enable = true;
  programs.virt-manager.enable = true;"#,
            "postgresql" => r#"  # PostgreSQL
  services.postgresql.enable = true;"#,
            "redis" => r#"  # Redis
  services.redis.servers."".enable = true;"#,
            "earlyoom" => r#"  # Early OOM
  services.earlyoom.enable = true;"#,
            "auto_upgrade" => r#"  # Auto Upgrade
  system.autoUpgrade.enable = true;"#,
            "auto_gc" => r#"  # Automatic Garbage Collection
  nix.gc = {
    automatic = true;
    dates = "weekly";
    options = "--delete-older-than 30d";
  };"#,
            "store_optimize" => r#"  # Store Optimization
  nix.settings.auto-optimise-store = true;"#,
            "tailscale" => r#"  # Tailscale VPN
  services.tailscale.enable = true;"#,
            _ => continue,
        };
        sections.push(section.to_string());
    }

    format!(
        r#"# NixOS Toolkit - Services Configuration
# DO NOT EDIT MANUALLY

{{ config, lib, pkgs, ... }}:

{{
{}
}}
"#,
        sections.join("\n\n")
    )
}

/// Read a template file from the templates directory
pub fn read_template(template_path: &str) -> Result<String, NixGenError> {
    let templates_dir = paths::templates_dir();
    let full_path = templates_dir.join(template_path);

    std::fs::read_to_string(&full_path).map_err(|e| {
        NixGenError::ReadError(format!("{}: {}", full_path.display(), e))
    })
}

/// Check if a template exists
pub fn template_exists(template_path: &str) -> bool {
    let templates_dir = paths::templates_dir();
    templates_dir.join(template_path).exists()
}

/// Generate a preview of what will be written with full options
pub fn generate_preview_full(options: &NixGenOptions) -> String {
    let mut preview = String::new();

    preview.push_str("=== Files to be written ===\n\n");

    // selected.nix
    preview.push_str(&format!("--- {} ---\n", paths::SELECTED_NIX));
    preview.push_str(&generate_selected_nix_full(options));
    preview.push('\n');

    // hostname.nix if needed
    if let Some(h) = options.hostname {
        preview.push_str(&format!("\n--- {} ---\n", paths::HOSTNAME_NIX));
        preview.push_str(&generate_hostname_nix(h));
    }

    // dns.nix if needed
    if !options.dns_servers.is_empty() {
        preview.push_str(&format!("\n--- {} ---\n", paths::DNS_NIX));
        preview.push_str(&generate_dns_nix(&options.dns_servers));
    }

    // users.nix if needed
    if !options.user_groups.is_empty() {
        if let Some(username) = options.username {
            preview.push_str(&format!("\n--- {} ---\n", paths::USERS_NIX));
            preview.push_str(&generate_user_groups_nix(username, &options.user_groups));
        }
    }

    // custom-packages.nix if needed
    if !options.custom_packages.is_empty() {
        preview.push_str(&format!("\n--- {} ---\n", paths::CUSTOM_PACKAGES_NIX));
        preview.push_str(&generate_custom_packages_nix(&options.custom_packages));
    }

    // hardware.nix if Bluetooth enabled
    if options.bluetooth_enabled {
        preview.push_str(&format!("\n--- {} ---\n", paths::HARDWARE_NIX));
        preview.push_str(&generate_hardware_nix(
            false, // nvidia_enabled - not implemented yet
            false, // nvidia_open
            false, // audio_pipewire
            true,  // bluetooth_enabled
            false, // tlp_enabled
            false, // thermald_enabled
        ));
    }

    // network.nix if network settings are configured
    if options.network_config.has_settings() {
        preview.push_str(&format!("\n--- {} ---\n", paths::NETWORK_NIX));
        let mut network_preview = String::from("# NixOS Toolkit - Network Configuration\n# DO NOT EDIT MANUALLY\n\n{ config, lib, pkgs, ... }:\n\n{\n");

        // Show firewall settings - generate directly
        let tcp_str = options.network_config.allowed_tcp_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let udp_str = options.network_config.allowed_udp_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        network_preview.push_str("  # Firewall\n");
        network_preview.push_str(&format!(
            "  networking.firewall = {{\n    enable = {};\n    allowedTCPPorts = [ {} ];\n    allowedUDPPorts = [ {} ];\n  }};\n",
            if options.network_config.firewall_enabled { "true" } else { "false" },
            tcp_str,
            udp_str
        ));

        // Show SSH settings - generate directly
        if options.network_config.ssh_enabled {
            network_preview.push_str("\n  # SSH\n");
            network_preview.push_str(&format!(
                "  services.openssh = {{\n    enable = true;\n    ports = [ {} ];\n    settings = {{\n      PasswordAuthentication = {};\n      PermitRootLogin = \"{}\";\n    }};\n  }};\n",
                options.network_config.ssh_port,
                if options.network_config.ssh_password_auth { "true" } else { "false" },
                options.network_config.ssh_root_login
            ));

            if options.network_config.fail2ban_enabled {
                network_preview.push_str("\n  # Fail2ban\n");
                network_preview.push_str("  services.fail2ban = {\n    enable = true;\n    jails.sshd = {\n      enabled = true;\n    };\n  };\n");
            }
        }

        // Show Tailscale
        if options.network_config.tailscale_enabled {
            network_preview.push_str("\n  # Tailscale VPN\n");
            network_preview.push_str("  services.tailscale.enable = true;\n");
        }

        network_preview.push_str("}\n");
        preview.push_str(&network_preview);
    }

    // services.nix if services are configured
    if options.services_config.has_settings() {
        preview.push_str(&format!("\n--- {} ---\n", paths::SERVICES_NIX));
        let enabled_services: Vec<&str> = options.services_config.enabled_services();
        preview.push_str(&generate_services_nix(&enabled_services));
    }

    // Profile template (if selected and exists)
    if let Some(p) = options.profile {
        if template_exists(&p.template) {
            if let Ok(content) = read_template(&p.template) {
                preview.push_str(&format!(
                    "\n--- /etc/nixos/nixos-toolkit/profiles/{}.nix ---\n",
                    p.id
                ));
                preview.push_str(&content);
            }
        }
    }

    // Bundle templates
    for bundle in &options.bundles {
        if template_exists(&bundle.template) {
            if let Ok(content) = read_template(&bundle.template) {
                preview.push_str(&format!(
                    "\n--- /etc/nixos/nixos-toolkit/bundles/{}.nix ---\n",
                    bundle.id
                ));
                preview.push_str(&content);
            }
        }
    }

    preview
}

/// Generate a preview of what will be written (legacy signature for compatibility)
pub fn generate_preview(
    profile: Option<&ProfileDef>,
    bundles: &[&BundleDef],
    hostname: Option<&str>,
) -> String {
    let options = NixGenOptions {
        profile,
        bundles: bundles.to_vec(),
        hostname,
        ..Default::default()
    };
    generate_preview_full(&options)
}
