//! Nix code generation utilities

use crate::actions::{BundleDef, ProfileDef};
use crate::config::paths;
use crate::ipc::{HardwareConfig, NetworkConfig, ServicesConfig};
use serde::{Deserialize, Serialize};
use std::path::Path;
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

/// Quote `s` as a Nix string literal.
#[must_use]
pub fn nix_escape_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '$' => out.push_str("\\$"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// True when `name` is a safe nixpkgs attribute path (dots allowed).
#[must_use]
pub fn is_nix_attrpath(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')) && !name.contains("..")
}

fn nix_permit_root_login(value: &str) -> &'static str {
    match value {
        "yes" => "yes",
        "prohibit-password" => "prohibit-password",
        _ => "no",
    }
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
    pub hardware_config: HardwareConfig,
}

impl<'a> NixGenOptions<'a> {
    /// Get the packages that should be enabled for a specific bundle.
    /// If bundle_packages contains an entry for this bundle, returns that list.
    /// Otherwise, returns all packages from the bundle definition.
    pub fn get_bundle_packages(&self, bundle: &BundleDef) -> Vec<String> {
        if let Some(packages) = self.bundle_packages.get(&bundle.id) {
            packages.clone()
        } else {
            bundle.packages.iter().map(|p| p.id.clone()).collect()
        }
    }

    /// Hardware config with the sibling `bluetooth_enabled` flag OR-ed in.
    #[must_use]
    pub fn effective_hardware(&self) -> HardwareConfig {
        self.hardware_config
            .clone()
            .with_bluetooth_or(self.bluetooth_enabled)
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

    // Add hardware config when any non-default hardware setting is set
    if options.effective_hardware().has_settings() {
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
            options
                .bundles
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
        if options.effective_hardware().bluetooth_enabled {
            "Enabled"
        } else {
            "Disabled"
        },
        if options.custom_packages.is_empty() {
            "None".to_string()
        } else {
            options.custom_packages.join(", ")
        },
        if options.network_config.has_settings() {
            "Configured"
        } else {
            "Default"
        },
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
  networking.hostName = lib.mkForce {};
}}
"#,
        nix_escape_string(hostname)
    )
}

/// Generate the dns.nix file content for custom DNS configuration
pub fn generate_dns_nix(servers: &[String]) -> String {
    if servers.is_empty() {
        return String::new();
    }

    let servers_str = servers
        .iter()
        .map(|s| format!("    {}", nix_escape_string(s)))
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
        .map(|g| nix_escape_string(g))
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
        nix_escape_string(username),
        groups_str
    )
}

/// Generate the custom-packages.nix file content for manually added packages
pub fn generate_custom_packages_nix(packages: &[String]) -> String {
    if packages.is_empty() {
        return String::new();
    }

    let packages_str = packages
        .iter()
        .filter(|p| is_nix_attrpath(p))
        .map(|p| format!("    {p}"))
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
        nix_permit_root_login(root_login)
    );

    if fail2ban {
        config.push_str(
            r#"
  services.fail2ban = {
    enable = true;
    jails.sshd = {
      enabled = true;
    };
  };
"#,
        );
    }

    config.push_str("}\n");
    config
}

fn nix_bool(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn power_profile_name(index: u8) -> &'static str {
    match index {
        1 => "performance",
        2 => "power-saver",
        _ => "balanced",
    }
}

/// Generate the hardware.nix file content from a full [`HardwareConfig`].
pub fn generate_hardware_nix(hw: &HardwareConfig) -> String {
    let mut sections = Vec::new();

    match hw.nvidia_driver {
        None => {}
        Some(3) => {
            sections.push(
                r#"  # NVIDIA GPU (nouveau)
  services.xserver.videoDrivers = [ "nouveau" ];
  hardware.graphics.enable = true;"#
                    .to_string(),
            );
        }
        Some(index) => {
            let (label, package) = match index {
                1 => ("beta", "config.boot.kernelPackages.nvidiaPackages.beta"),
                2 => (
                    "nvidia-open",
                    "config.boot.kernelPackages.nvidiaPackages.latest",
                ),
                _ => ("stable", "config.boot.kernelPackages.nvidiaPackages.stable"),
            };
            sections.push(format!(
                r#"  # NVIDIA GPU ({label})
  services.xserver.videoDrivers = [ "nvidia" ];
  hardware.nvidia = {{
    package = {package};
    modesetting.enable = {};
    powerManagement.enable = {};
    open = {};
  }};
  hardware.graphics.enable = true;"#,
                nix_bool(hw.nvidia_modesetting),
                nix_bool(hw.nvidia_powermanagement),
                nix_bool(hw.nvidia_open),
            ));
        }
    }

    match hw.audio_server {
        1 => {
            sections.push(
                r#"  # PulseAudio
  services.pulseaudio.enable = true;
  services.pipewire.enable = false;"#
                    .to_string(),
            );
        }
        2 => {
            sections.push(
                r#"  # Audio disabled
  services.pipewire.enable = false;
  services.pulseaudio.enable = false;"#
                    .to_string(),
            );
        }
        _ => {
            // PipeWire (0). Emit the existing block when the user opted in
            // via low-latency, so a bluetooth-only apply does not force it.
            if hw.audio_lowlatency {
                sections.push(
                    r#"  # PipeWire Audio
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;
    jack.enable = true;
    extraConfig.pipewire."92-low-latency" = {
      "context.properties" = {
        "default.clock.rate" = 48000;
        "default.clock.quantum" = 32;
        "default.clock.min-quantum" = 32;
        "default.clock.max-quantum" = 32;
      };
    };
  };
  security.rtkit.enable = true;"#
                        .to_string(),
                );
            }
        }
    }

    if hw.bluetooth_enabled {
        sections.push(format!(
            r#"  # Bluetooth
  hardware.bluetooth = {{
    enable = true;
    powerOnBoot = {};
  }};
  services.blueman.enable = true;"#,
            nix_bool(hw.bluetooth_autopower)
        ));
    }

    if hw.tlp_enabled {
        sections.push(
            r#"  # TLP Power Management
  services.tlp = {
    enable = true;
    settings = {
      CPU_SCALING_GOVERNOR_ON_AC = "performance";
      CPU_SCALING_GOVERNOR_ON_BAT = "powersave";
    };
  };
  services.power-profiles-daemon.enable = false;"#
                .to_string(),
        );
    } else if hw.power_profile != 0 {
        // Omit Balanced (0): GTK never emitted PPD, so bluetooth-only apply must not enable it.
        let profile = power_profile_name(hw.power_profile);
        sections.push(format!(
            r#"  # Power profile ({profile})
  services.power-profiles-daemon.enable = true;
  systemd.services.nixos-toolkit-power-profile = {{
    description = "Set power-profiles-daemon profile";
    wantedBy = [ "multi-user.target" ];
    after = [ "power-profiles-daemon.service" ];
    serviceConfig.Type = "oneshot";
    script = "${{pkgs.power-profiles-daemon}}/bin/powerprofilesctl set {profile}";
  }};"#
        ));
    }

    if hw.thermald_enabled {
        sections.push(
            r#"  # Thermal Management
  services.thermald.enable = true;"#
                .to_string(),
        );
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
            "printing" => {
                r#"  # Printing
  services.printing.enable = true;
  services.avahi.enable = true;
  services.avahi.nssmdns4 = true;"#
            }
            "avahi" => {
                r#"  # Avahi/mDNS
  services.avahi = {
    enable = true;
    nssmdns4 = true;
    publish.enable = true;
  };"#
            }
            "fwupd" => {
                r#"  # Firmware Updates
  services.fwupd.enable = true;"#
            }
            "upower" => {
                r#"  # UPower
  services.upower.enable = true;"#
            }
            "networkmanager" => {
                r#"  # NetworkManager
  networking.networkmanager.enable = true;"#
            }
            "resolved" => {
                r#"  # systemd-resolved
  services.resolved.enable = true;"#
            }
            "rustdesk" => {
                r#"  # RustDesk client
  environment.systemPackages = with pkgs; [ rustdesk ];"#
            }
            "syncthing" => {
                r#"  # Syncthing
  services.syncthing.enable = true;"#
            }
            "locate" => {
                r#"  # Locate Database
  services.locate = {
    enable = true;
    package = pkgs.plocate;
  };"#
            }
            "flatpak" => {
                r#"  # Flatpak
  services.flatpak.enable = true;
  xdg.portal.enable = true;"#
            }
            "gnome_keyring" => {
                r#"  # GNOME Keyring
  services.gnome.gnome-keyring.enable = true;"#
            }
            "gnome_tweaks" => {
                r#"  # GNOME Tweaks
  environment.systemPackages = with pkgs; [ gnome-tweaks ];"#
            }
            "dconf" => {
                r#"  # dconf
  programs.dconf.enable = true;"#
            }
            "docker" => {
                r#"  # Docker
  virtualisation.docker.enable = true;"#
            }
            "libvirtd" => {
                r#"  # libvirtd
  virtualisation.libvirtd.enable = true;
  programs.virt-manager.enable = true;"#
            }
            "postgresql" => {
                r#"  # PostgreSQL
  services.postgresql.enable = true;"#
            }
            "redis" => {
                r#"  # Redis
  services.redis.servers."".enable = true;"#
            }
            "earlyoom" => {
                r#"  # Early OOM
  services.earlyoom.enable = true;"#
            }
            "auto_upgrade" => {
                r#"  # Auto Upgrade
  system.autoUpgrade.enable = true;"#
            }
            "auto_gc" => {
                r#"  # Automatic Garbage Collection
  nix.gc = {
    automatic = true;
    dates = "weekly";
    options = "--delete-older-than 30d";
  };"#
            }
            "store_optimize" => {
                r#"  # Store Optimization
  nix.settings.auto-optimise-store = true;"#
            }
            "tailscale" => {
                r#"  # Tailscale VPN
  services.tailscale.enable = true;"#
            }
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

/// Read a template file from an explicit templates root.
pub fn read_template_from(
    templates_dir: &Path,
    template_path: &str,
) -> Result<String, NixGenError> {
    let full_path = templates_dir.join(template_path);

    std::fs::read_to_string(&full_path)
        .map_err(|e| NixGenError::ReadError(format!("{}: {}", full_path.display(), e)))
}

/// Read a template file from the templates directory
pub fn read_template(template_path: &str) -> Result<String, NixGenError> {
    read_template_from(&paths::templates_dir(), template_path)
}

/// Check if a template exists under an explicit templates root.
pub fn template_exists_in(templates_dir: &Path, template_path: &str) -> bool {
    templates_dir.join(template_path).exists()
}

/// Check if a template exists
pub fn template_exists(template_path: &str) -> bool {
    template_exists_in(&paths::templates_dir(), template_path)
}

/// Generate a preview of what will be written with full options
pub fn generate_preview_full(options: &NixGenOptions) -> String {
    generate_preview_full_from(options, &paths::templates_dir())
}

/// [`generate_preview_full`] using `templates_dir` instead of the process env.
pub fn generate_preview_full_from(options: &NixGenOptions, templates_dir: &Path) -> String {
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

    // hardware.nix when any non-default hardware setting is set
    let hardware = options.effective_hardware();
    if hardware.has_settings() {
        preview.push_str(&format!("\n--- {} ---\n", paths::HARDWARE_NIX));
        preview.push_str(&generate_hardware_nix(&hardware));
    }

    // network.nix if network settings are configured
    if options.network_config.has_settings() {
        preview.push_str(&format!("\n--- {} ---\n", paths::NETWORK_NIX));
        let mut network_preview = String::from("# NixOS Toolkit - Network Configuration\n# DO NOT EDIT MANUALLY\n\n{ config, lib, pkgs, ... }:\n\n{\n");

        // Show firewall settings - generate directly
        let tcp_str = options
            .network_config
            .allowed_tcp_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let udp_str = options
            .network_config
            .allowed_udp_ports
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
                nix_permit_root_login(&options.network_config.ssh_root_login)
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
        if template_exists_in(templates_dir, &p.template) {
            if let Ok(content) = read_template_from(templates_dir, &p.template) {
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
        if template_exists_in(templates_dir, &bundle.template) {
            if let Ok(content) = read_template_from(templates_dir, &bundle.template) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::HardwareConfig;

    fn hw(update: impl FnOnce(&mut HardwareConfig)) -> HardwareConfig {
        let mut config = HardwareConfig::default();
        update(&mut config);
        config
    }

    #[test]
    fn nvidia_stable_uses_package_and_field_flags() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.nvidia_driver = Some(0);
            c.nvidia_modesetting = true;
            c.nvidia_powermanagement = false;
            c.nvidia_open = false;
        }));
        assert!(nix.contains("# NVIDIA GPU (stable)"));
        assert!(nix.contains("videoDrivers = [ \"nvidia\" ]"));
        assert!(nix.contains("nvidiaPackages.stable"));
        assert!(nix.contains("modesetting.enable = true;"));
        assert!(nix.contains("powerManagement.enable = false;"));
        assert!(nix.contains("open = false;"));
        assert!(!nix.contains("nouveau"));
    }

    #[test]
    fn nvidia_beta_uses_beta_package() {
        let nix = generate_hardware_nix(&hw(|c| c.nvidia_driver = Some(1)));
        assert!(nix.contains("# NVIDIA GPU (beta)"));
        assert!(nix.contains("nvidiaPackages.beta"));
        assert!(nix.contains("videoDrivers = [ \"nvidia\" ]"));
        assert!(nix.contains("modesetting.enable = true;"));
        assert!(nix.contains("powerManagement.enable = false;"));
        assert!(nix.contains("open = false;"));
    }

    #[test]
    fn nvidia_open_driver_index() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.nvidia_driver = Some(2);
            c.nvidia_open = true;
        }));
        assert!(nix.contains("# NVIDIA GPU (nvidia-open)"));
        assert!(nix.contains("nvidiaPackages.latest"));
        assert!(nix.contains("videoDrivers = [ \"nvidia\" ]"));
        assert!(nix.contains("open = true;"));
        assert!(!nix.contains("nouveau"));
    }

    #[test]
    fn nvidia_nouveau_has_no_hardware_nvidia_block() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.nvidia_driver = Some(3);
            c.nvidia_modesetting = true;
            c.nvidia_open = true;
        }));
        assert!(nix.contains("videoDrivers = [ \"nouveau\" ]"));
        assert!(!nix.contains("hardware.nvidia"));
        assert!(!nix.contains("nvidiaPackages"));
    }

    #[test]
    fn nvidia_none_omits_gpu_section() {
        let nix = generate_hardware_nix(&HardwareConfig::default());
        assert!(!nix.contains("videoDrivers"));
        assert!(!nix.contains("hardware.nvidia"));
        assert!(!nix.contains("power-profiles-daemon"));
    }

    #[test]
    fn nvidia_modesetting_false_is_honored() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.nvidia_driver = Some(0);
            c.nvidia_modesetting = false;
        }));
        assert!(nix.contains("modesetting.enable = false;"));
        assert!(!nix.contains("modesetting.enable = true;"));
    }

    #[test]
    fn pulseaudio_disables_pipewire() {
        let nix = generate_hardware_nix(&hw(|c| c.audio_server = 1));
        assert!(nix.contains("services.pulseaudio.enable = true;"));
        assert!(nix.contains("services.pipewire.enable = false;"));
        assert!(!nix.contains("services.pipewire = {"));
    }

    #[test]
    fn audio_none_disables_servers() {
        let nix = generate_hardware_nix(&hw(|c| c.audio_server = 2));
        assert!(nix.contains("services.pipewire.enable = false;"));
        assert!(nix.contains("services.pulseaudio.enable = false;"));
    }

    #[test]
    fn pipewire_lowlatency_adds_quantum() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.audio_server = 0;
            c.audio_lowlatency = true;
        }));
        assert!(nix.contains("services.pipewire = {"));
        assert!(nix.contains("default.clock.quantum"));
        assert!(nix.contains("92-low-latency"));
    }

    #[test]
    fn lowlatency_skipped_when_not_pipewire() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.audio_server = 1;
            c.audio_lowlatency = true;
        }));
        assert!(!nix.contains("92-low-latency"));
        assert!(nix.contains("services.pulseaudio.enable = true;"));
    }

    #[test]
    fn bluetooth_autopower_false() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.bluetooth_enabled = true;
            c.bluetooth_autopower = false;
        }));
        assert!(nix.contains("hardware.bluetooth"));
        assert!(nix.contains("powerOnBoot = false;"));
        assert!(!nix.contains("powerOnBoot = true;"));
        assert!(!nix.contains("power-profiles-daemon"));
    }

    #[test]
    fn bluetooth_autopower_true() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.bluetooth_enabled = true;
            c.bluetooth_autopower = true;
        }));
        assert!(nix.contains("powerOnBoot = true;"));
    }

    #[test]
    fn tlp_disables_power_profiles_daemon() {
        let nix = generate_hardware_nix(&hw(|c| {
            c.tlp_enabled = true;
            c.power_profile = 1;
        }));
        assert!(nix.contains("services.tlp"));
        assert!(nix.contains("services.power-profiles-daemon.enable = false;"));
        assert!(!nix.contains("powerprofilesctl"));
        assert!(!nix.contains("power-profiles-daemon.enable = true;"));

        let tlp_balanced = generate_hardware_nix(&hw(|c| c.tlp_enabled = true));
        assert!(tlp_balanced.contains("services.tlp"));
        assert!(tlp_balanced.contains("services.power-profiles-daemon.enable = false;"));
        assert!(!tlp_balanced.contains("power-profiles-daemon.enable = true;"));
    }

    #[test]
    fn power_profile_maps_when_tlp_off() {
        let balanced = generate_hardware_nix(&HardwareConfig::default());
        assert!(!balanced.contains("power-profiles-daemon"));
        assert!(!balanced.contains("powerprofilesctl"));

        let performance = generate_hardware_nix(&hw(|c| c.power_profile = 1));
        assert!(performance.contains("power-profiles-daemon.enable = true;"));
        assert!(performance.contains("powerprofilesctl set performance"));
        assert!(!performance.contains("services.tlp"));

        let saver = generate_hardware_nix(&hw(|c| c.power_profile = 2));
        assert!(saver.contains("power-profiles-daemon.enable = true;"));
        assert!(saver.contains("powerprofilesctl set power-saver"));
        assert!(!saver.contains("services.tlp"));
    }

    #[test]
    fn bluetooth_audio_nvidia_only_omit_power_profiles_daemon() {
        let bluetooth = generate_hardware_nix(&hw(|c| c.bluetooth_enabled = true));
        assert!(bluetooth.contains("hardware.bluetooth"));
        assert!(!bluetooth.contains("power-profiles-daemon"));

        let pulse = generate_hardware_nix(&hw(|c| c.audio_server = 1));
        assert!(pulse.contains("services.pulseaudio.enable = true;"));
        assert!(!pulse.contains("power-profiles-daemon"));

        let nvidia = generate_hardware_nix(&hw(|c| c.nvidia_driver = Some(0)));
        assert!(nvidia.contains("videoDrivers"));
        assert!(!nvidia.contains("power-profiles-daemon"));
    }

    #[test]
    fn thermald_section() {
        let nix = generate_hardware_nix(&hw(|c| c.thermald_enabled = true));
        assert!(nix.contains("services.thermald.enable = true;"));
    }

    #[test]
    fn selected_nix_imports_hardware_for_non_default_not_only_bluetooth() {
        let options = NixGenOptions {
            hardware_config: hw(|c| c.nvidia_driver = Some(0)),
            ..Default::default()
        };
        let selected = generate_selected_nix_full(&options);
        assert!(selected.contains("./hardware.nix"));

        let bluetooth_only = NixGenOptions {
            bluetooth_enabled: true,
            ..Default::default()
        };
        assert!(generate_selected_nix_full(&bluetooth_only).contains("./hardware.nix"));

        let empty = NixGenOptions::default();
        assert!(!generate_selected_nix_full(&empty).contains("./hardware.nix"));
    }

    #[test]
    fn preview_includes_hardware_nix_for_nvidia() {
        let options = NixGenOptions {
            hardware_config: hw(|c| c.audio_server = 1),
            ..Default::default()
        };
        let preview = generate_preview_full(&options);
        assert!(preview.contains("hardware.nix"));
        assert!(preview.contains("services.pulseaudio.enable = true;"));
    }

    #[test]
    fn nix_escape_string_quotes_and_dollars() {
        assert_eq!(nix_escape_string("nixos"), "\"nixos\"");
        assert_eq!(nix_escape_string("foo\"bar"), "\"foo\\\"bar\"");
        assert_eq!(nix_escape_string("a${b}"), "\"a\\${b}\"");
    }

    #[test]
    fn user_groups_quotes_hyphenated_username() {
        let nix = generate_user_groups_nix("gosh-1", &["libvirtd".into()]);
        assert!(nix.contains("users.users.\"gosh-1\""));
        assert!(nix.contains("extraGroups = [ \"libvirtd\" ];"));
        assert!(!nix.contains("users.users.gosh-1"));
    }

    #[test]
    fn hostname_is_quoted_nix_string() {
        let nix = generate_hostname_nix("desk-1");
        assert!(nix.contains("networking.hostName = lib.mkForce \"desk-1\";"));
    }

    #[test]
    fn locate_omits_removed_localuser() {
        let nix = generate_services_nix(&["locate"]);
        assert!(nix.contains("services.locate"));
        assert!(nix.contains("package = pkgs.plocate;"));
        assert!(!nix.contains("localuser"));
    }

    #[test]
    fn rustdesk_is_client_package_not_server() {
        let nix = generate_services_nix(&["rustdesk"]);
        assert!(nix.contains("environment.systemPackages = with pkgs; [ rustdesk ];"));
        assert!(!nix.contains("rustdesk-server"));
    }

    #[test]
    fn ssh_root_login_is_allowlisted() {
        let evil = generate_ssh_nix(
            true,
            22,
            false,
            "yes\";\n  networking.hostName = \"pwned",
            false,
        );
        assert!(evil.contains("PermitRootLogin = \"no\";"));
        assert!(!evil.contains("pwned"));
        let ok = generate_ssh_nix(true, 22, false, "prohibit-password", false);
        assert!(ok.contains("PermitRootLogin = \"prohibit-password\";"));
    }
}
