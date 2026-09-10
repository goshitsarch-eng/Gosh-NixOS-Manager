//! Nix file generation

use common::actions::{default_bundles, default_profiles};
use common::config::paths;
use common::ipc::{GeneratedFile, HardwareConfig, NetworkConfig, ServicesConfig};
use common::nix::{
    generate_custom_packages_nix, generate_dns_nix, generate_hardware_nix, generate_hostname_nix,
    generate_selected_nix_full, generate_services_nix, generate_user_groups_nix, read_template,
    NixGenOptions,
};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Generate all configuration files
#[allow(clippy::too_many_arguments)]
pub fn generate_all_files(
    selected_profile: &Option<String>,
    enabled_bundles: &[String],
    bundle_packages: &HashMap<String, Vec<String>>,
    hostname: Option<&str>,
    dns_servers: &[String],
    user_groups: &[String],
    username: Option<&str>,
    bluetooth_enabled: bool,
    custom_packages: &[String],
    network_config: &NetworkConfig,
    services_config: &ServicesConfig,
    hardware_config: &HardwareConfig,
    dry_run: bool,
) -> anyhow::Result<Vec<GeneratedFile>> {
    let mut files = Vec::new();

    let profiles = default_profiles();
    let bundles = default_bundles();

    // Find selected profile
    let profile = selected_profile
        .as_ref()
        .and_then(|id| profiles.iter().find(|p| &p.id == id));

    // Find enabled bundles
    let enabled: Vec<_> = bundles
        .iter()
        .filter(|b| enabled_bundles.contains(&b.id))
        .collect();

    let hardware_config = hardware_config.clone().with_bluetooth_or(bluetooth_enabled);

    // Build options for Nix generation
    let options = NixGenOptions {
        profile,
        bundles: enabled.clone(),
        bundle_packages: bundle_packages.clone(),
        hostname,
        dns_servers: dns_servers.to_vec(),
        user_groups: user_groups.to_vec(),
        username,
        bluetooth_enabled: hardware_config.bluetooth_enabled,
        custom_packages: custom_packages.to_vec(),
        network_config: network_config.clone(),
        services_config: services_config.clone(),
        hardware_config: hardware_config.clone(),
    };

    // Generate selected.nix
    let selected_content = generate_selected_nix_full(&options);
    files.push(GeneratedFile {
        path: paths::SELECTED_NIX.to_string(),
        content: selected_content.clone(),
    });

    if !dry_run {
        atomic_write(paths::SELECTED_NIX, &selected_content)?;
    }

    // Generate hostname.nix if needed
    if let Some(h) = hostname {
        let hostname_content = generate_hostname_nix(h);
        files.push(GeneratedFile {
            path: paths::HOSTNAME_NIX.to_string(),
            content: hostname_content.clone(),
        });

        if !dry_run {
            atomic_write(paths::HOSTNAME_NIX, &hostname_content)?;
        }
    }

    // Generate dns.nix if DNS servers are configured
    if !dns_servers.is_empty() {
        let dns_content = generate_dns_nix(dns_servers);
        files.push(GeneratedFile {
            path: paths::DNS_NIX.to_string(),
            content: dns_content.clone(),
        });

        if !dry_run {
            atomic_write(paths::DNS_NIX, &dns_content)?;
        }
    }

    // Generate users.nix if user groups are configured
    if let Some(user) = username {
        if !user_groups.is_empty() {
            let users_content = generate_user_groups_nix(user, user_groups);
            files.push(GeneratedFile {
                path: paths::USERS_NIX.to_string(),
                content: users_content.clone(),
            });

            if !dry_run {
                atomic_write(paths::USERS_NIX, &users_content)?;
            }
        }
    }

    // Generate hardware.nix when any non-default hardware setting is set
    if hardware_config.has_settings() {
        let hardware_content = generate_hardware_nix(&hardware_config);
        files.push(GeneratedFile {
            path: paths::HARDWARE_NIX.to_string(),
            content: hardware_content.clone(),
        });

        if !dry_run {
            atomic_write(paths::HARDWARE_NIX, &hardware_content)?;
        }
    }

    // Copy profile template if selected
    if let Some(p) = profile {
        let dest_path = format!("{}/{}.nix", paths::PROFILES_DIR, p.id);
        if let Ok(content) = read_template(&p.template) {
            files.push(GeneratedFile {
                path: dest_path.clone(),
                content: content.clone(),
            });

            if !dry_run {
                atomic_write(&dest_path, &content)?;
            }
        } else {
            // Use fallback template
            let fallback = generate_fallback_profile(&p.id);
            files.push(GeneratedFile {
                path: dest_path.clone(),
                content: fallback.clone(),
            });

            if !dry_run {
                atomic_write(&dest_path, &fallback)?;
            }
        }
    }

    // Copy bundle templates
    for bundle in &enabled {
        let dest_path = format!("{}/{}.nix", paths::BUNDLES_DIR, bundle.id);

        // Get the packages to use for this bundle
        // If bundle_packages has an entry, use that (user customized); otherwise use all packages
        let packages_to_use: Vec<String> =
            if let Some(custom_packages) = bundle_packages.get(&bundle.id) {
                custom_packages.clone()
            } else {
                bundle.packages.iter().map(|p| p.id.clone()).collect()
            };

        // If user has customized packages, always generate a custom template
        // This ensures the generated Nix file reflects the user's package selection
        let has_custom_packages = bundle_packages.contains_key(&bundle.id);

        if !has_custom_packages {
            // No customization - try to use the pre-made template
            if let Ok(content) = read_template(&bundle.template) {
                files.push(GeneratedFile {
                    path: dest_path.clone(),
                    content: content.clone(),
                });

                if !dry_run {
                    atomic_write(&dest_path, &content)?;
                }
                continue;
            }
        }

        // Either customized packages or no template available - generate fallback
        let fallback = generate_fallback_bundle(&bundle.id, &packages_to_use);
        files.push(GeneratedFile {
            path: dest_path.clone(),
            content: fallback.clone(),
        });

        if !dry_run {
            atomic_write(&dest_path, &fallback)?;
        }
    }

    // Generate custom-packages.nix if custom packages exist
    if !custom_packages.is_empty() {
        let custom_content = generate_custom_packages_nix(custom_packages);
        files.push(GeneratedFile {
            path: paths::CUSTOM_PACKAGES_NIX.to_string(),
            content: custom_content.clone(),
        });

        if !dry_run {
            atomic_write(paths::CUSTOM_PACKAGES_NIX, &custom_content)?;
        }
    }

    // Generate network.nix if network settings are configured
    if network_config.has_settings() {
        // Generate a combined network config that includes firewall, SSH, and VPN settings
        let mut network_content = String::from("# NixOS Toolkit - Network Configuration\n# DO NOT EDIT MANUALLY\n\n{ config, lib, pkgs, ... }:\n\n{\n");

        // Firewall configuration - generate directly
        let tcp_str = network_config
            .allowed_tcp_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let udp_str = network_config
            .allowed_udp_ports
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        network_content.push_str("  # Firewall\n");
        network_content.push_str(&format!(
            "  networking.firewall = {{\n    enable = {};\n    allowedTCPPorts = [ {} ];\n    allowedUDPPorts = [ {} ];\n  }};\n",
            if network_config.firewall_enabled { "true" } else { "false" },
            tcp_str,
            udp_str
        ));

        // SSH configuration - generate directly
        if network_config.ssh_enabled {
            network_content.push_str("\n  # SSH\n");
            network_content.push_str(&format!(
                "  services.openssh = {{\n    enable = true;\n    ports = [ {} ];\n    settings = {{\n      PasswordAuthentication = {};\n      PermitRootLogin = \"{}\";\n    }};\n  }};\n",
                network_config.ssh_port,
                if network_config.ssh_password_auth { "true" } else { "false" },
                network_config.ssh_root_login
            ));

            if network_config.fail2ban_enabled {
                network_content.push_str("\n  # Fail2ban\n");
                network_content.push_str("  services.fail2ban = {\n    enable = true;\n    jails.sshd = {\n      enabled = true;\n    };\n  };\n");
            }
        }

        // Tailscale VPN
        if network_config.tailscale_enabled {
            network_content.push_str("\n  # Tailscale VPN\n");
            network_content.push_str("  services.tailscale.enable = true;\n");
        }

        network_content.push_str("}\n");

        files.push(GeneratedFile {
            path: paths::NETWORK_NIX.to_string(),
            content: network_content.clone(),
        });

        if !dry_run {
            atomic_write(paths::NETWORK_NIX, &network_content)?;
        }
    }

    // Generate services.nix if services are configured
    if services_config.has_settings() {
        let enabled_services: Vec<&str> = services_config.enabled_services();
        let services_content = generate_services_nix(&enabled_services);

        files.push(GeneratedFile {
            path: paths::SERVICES_NIX.to_string(),
            content: services_content.clone(),
        });

        if !dry_run {
            atomic_write(paths::SERVICES_NIX, &services_content)?;
        }
    }

    Ok(files)
}

/// Atomic write to a file (write to temp, then rename)
fn atomic_write(path: &str, content: &str) -> anyhow::Result<()> {
    let path = Path::new(path);

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to temp file
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, content)?;

    // Rename to final path
    fs::rename(&temp_path, path)?;

    // Verify the write succeeded by reading back
    let written = fs::read_to_string(path)?;
    if written != content {
        anyhow::bail!(
            "Write verification failed for {}: content mismatch",
            path.display()
        );
    }

    tracing::info!("Wrote and verified {}", path.display());
    Ok(())
}

/// Generate a fallback profile template
fn generate_fallback_profile(id: &str) -> String {
    match id {
        "gnome" => r#"# GNOME Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.sddm.enable = lib.mkForce false;
  services.xserver.displayManager.lightdm.enable = lib.mkForce false;
  # Enable GDM
  services.displayManager.gdm.enable = lib.mkForce true;
  services.displayManager.gdm.wayland = true;
  services.desktopManager.gnome.enable = true;
  services.gnome.core-utilities.enable = true;
  programs.dconf.enable = true;
  environment.systemPackages = with pkgs; [ gnome-tweaks dconf-editor gnome-extension-manager ];
}
"#
        .to_string(),
        "kde" => r#"# KDE Plasma 6 Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.xserver.displayManager.lightdm.enable = lib.mkForce false;
  # Enable SDDM
  services.displayManager.sddm.enable = lib.mkForce true;
  services.displayManager.sddm.wayland.enable = true;
  # Disable other desktop environments
  services.desktopManager.gnome.enable = lib.mkForce false;
  services.xserver.desktopManager.gnome.enable = lib.mkForce false;
  # Enable KDE Plasma 6
  services.desktopManager.plasma6.enable = true;
  programs.kdeconnect.enable = true;
  # Override ssh askPassword (conflicts with GNOME's seahorse)
  programs.ssh.askPassword = lib.mkForce "${pkgs.kdePackages.ksshaskpass}/bin/ksshaskpass";
  programs.seahorse.enable = lib.mkForce false;
  environment.systemPackages = with pkgs; [ kdePackages.kate kdePackages.konsole kdePackages.dolphin ];
}
"#
        .to_string(),
        "xfce" => r#"# XFCE Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  # Enable LightDM
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;
  services.xserver.desktopManager.xfce.enable = true;
  programs.nm-applet.enable = true;
  xdg.portal.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
}
"#
        .to_string(),
        "mate" => r#"# MATE Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  # Enable LightDM
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;
  services.xserver.desktopManager.mate.enable = true;
  programs.nm-applet.enable = true;
  xdg.portal.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
}
"#
        .to_string(),
        "cinnamon" => r#"# Cinnamon Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  # Enable LightDM
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;
  services.xserver.desktopManager.cinnamon.enable = true;
}
"#
        .to_string(),
        "pantheon" => r#"# Pantheon Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  # Enable LightDM
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;
  services.xserver.desktopManager.pantheon.enable = true;
  services.pantheon.apps.enable = true;
  xdg.portal.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk pkgs.xdg-desktop-portal-pantheon ];
}
"#
        .to_string(),
        "cosmic" => r#"# COSMIC Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  services.xserver.displayManager.lightdm.enable = lib.mkForce false;
  # Enable COSMIC greeter
  services.displayManager.cosmic-greeter.enable = lib.mkForce true;
  services.desktopManager.cosmic.enable = true;
  hardware.graphics.enable = true;
  xdg.portal.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-cosmic ];
}
"#
        .to_string(),
        "hyprland" => r#"# Hyprland Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  programs.hyprland.enable = true;
  programs.hyprland.xwayland.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.xserver.displayManager.lightdm.enable = lib.mkForce false;
  # Enable SDDM
  services.displayManager.sddm.enable = lib.mkForce true;
  services.displayManager.sddm.wayland.enable = true;
  xdg.portal.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-hyprland pkgs.xdg-desktop-portal-gtk ];
  environment.systemPackages = with pkgs; [ waybar wofi dunst kitty ];
}
"#
        .to_string(),
        "sway" => r#"# Sway Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  programs.sway.enable = true;
  programs.sway.wrapperFeatures.gtk = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.xserver.displayManager.lightdm.enable = lib.mkForce false;
  # Enable SDDM
  services.displayManager.sddm.enable = lib.mkForce true;
  services.displayManager.sddm.wayland.enable = true;
  xdg.portal.enable = true;
  xdg.portal.wlr.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
  environment.systemPackages = with pkgs; [ waybar wofi foot ];
}
"#
        .to_string(),
        "i3" => r#"# i3 Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  services.xserver.windowManager.i3.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  # Enable LightDM
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;
  environment.systemPackages = with pkgs; [ dmenu rofi i3status alacritty ];
}
"#
        .to_string(),
        "budgie" => r#"# Budgie Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  # Enable LightDM
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;
  services.xserver.desktopManager.budgie.enable = true;
  xdg.portal.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
}
"#
        .to_string(),
        "lxqt" => r#"# LXQt Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.xserver.displayManager.lightdm.enable = lib.mkForce false;
  # Enable SDDM
  services.displayManager.sddm.enable = lib.mkForce true;
  services.xserver.desktopManager.lxqt.enable = true;
  xdg.portal.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-lxqt ];
}
"#
        .to_string(),
        "enlightenment" => r#"# Enlightenment Desktop Profile
# Generated by NixOS Toolkit

{ config, lib, pkgs, ... }:

{
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  # Enable LightDM
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;
  services.xserver.desktopManager.enlightenment.enable = true;
  xdg.portal.enable = true;
  xdg.portal.extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
}
"#
        .to_string(),
        _ => format!(
            r#"# {} Desktop Profile
# Generated by NixOS Toolkit

{{ config, lib, pkgs, ... }}:

{{
  # Profile: {}
  services.xserver.enable = true;
  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;
}}
"#,
            id, id
        ),
    }
}

/// Generate a fallback bundle
fn generate_fallback_bundle(id: &str, packages: &[String]) -> String {
    let pkg_list = packages
        .iter()
        .map(|p| format!("    {}", p))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"# {} Bundle
# Generated by NixOS Toolkit

{{ config, lib, pkgs, ... }}:

{{
  environment.systemPackages = with pkgs; [
{}
  ];
}}
"#,
        id, pkg_list
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::ipc::HardwareConfig;
    use std::collections::HashMap;

    fn dry_run(bluetooth: bool, hardware: &HardwareConfig) -> anyhow::Result<Vec<GeneratedFile>> {
        generate_all_files(
            &None,
            &[],
            &HashMap::new(),
            None,
            &[],
            &[],
            None,
            bluetooth,
            &[],
            &NetworkConfig::default(),
            &ServicesConfig::default(),
            hardware,
            true,
        )
    }

    #[test]
    fn dry_run_emits_hardware_nix_for_nvidia() {
        let hw = HardwareConfig {
            nvidia_driver: Some(0),
            ..HardwareConfig::default()
        };
        let files = dry_run(false, &hw).unwrap();
        let hardware = files
            .iter()
            .find(|f| f.path.contains("hardware.nix"))
            .expect("hardware.nix");
        assert!(hardware.content.contains("nvidiaPackages.stable"));
        assert!(!hardware
            .content
            .contains("power-profiles-daemon.enable = true;"));
        let selected = files
            .iter()
            .find(|f| f.path.contains("selected.nix"))
            .expect("selected.nix");
        assert!(selected.content.contains("./hardware.nix"));
    }

    #[test]
    fn dry_run_ors_sibling_bluetooth() {
        let files = dry_run(true, &HardwareConfig::default()).unwrap();
        let hardware = files
            .iter()
            .find(|f| f.path.contains("hardware.nix"))
            .expect("hardware.nix");
        assert!(hardware.content.contains("hardware.bluetooth"));
        assert!(!hardware
            .content
            .contains("power-profiles-daemon.enable = true;"));
    }

    #[test]
    fn dry_run_skips_hardware_when_default() {
        let files = dry_run(false, &HardwareConfig::default()).unwrap();
        assert!(!files.iter().any(|f| f.path.contains("hardware.nix")));
    }
}
