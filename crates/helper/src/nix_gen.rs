//! Nix file generation

use common::actions::{default_bundles, default_profiles};
use common::config::paths;
use common::ipc::{GeneratedFile, HardwareConfig, NetworkConfig, ServicesConfig};
use common::nix::{
    generate_custom_packages_nix, generate_dns_nix, generate_fallback_bundle,
    generate_hardware_nix, generate_hostname_nix, generate_network_nix, generate_selected_nix_full,
    generate_services_nix, generate_unfree_nix, generate_user_groups_nix, needs_allow_unfree,
    read_template, NixGenOptions,
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
    let mut keep_snippets: Vec<&'static str> = Vec::new();

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
        keep_snippets.push("hostname.nix");
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
        keep_snippets.push("dns.nix");
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
            keep_snippets.push("users.nix");
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
        keep_snippets.push("hardware.nix");
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

        let packages_to_use = options.get_bundle_packages(bundle);
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
        keep_snippets.push("custom-packages.nix");
    }

    // Generate network.nix if network settings are configured
    if network_config.has_settings() {
        let network_content = generate_network_nix(network_config);

        files.push(GeneratedFile {
            path: paths::NETWORK_NIX.to_string(),
            content: network_content.clone(),
        });

        if !dry_run {
            atomic_write(paths::NETWORK_NIX, &network_content)?;
        }
        keep_snippets.push("network.nix");
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
        keep_snippets.push("services.nix");
    }

    if needs_allow_unfree(&options) {
        let unfree_content = generate_unfree_nix();
        files.push(GeneratedFile {
            path: paths::UNFREE_NIX.to_string(),
            content: unfree_content.clone(),
        });

        if !dry_run {
            atomic_write(paths::UNFREE_NIX, &unfree_content)?;
        }
        keep_snippets.push("unfree.nix");
    }

    if !dry_run {
        cleanup_stale_managed_files(
            Path::new(paths::PROFILES_DIR),
            Path::new(paths::BUNDLES_DIR),
            Path::new(paths::STATE_DIR),
            selected_profile.as_deref(),
            enabled_bundles,
            &keep_snippets,
        )?;
    }

    Ok(files)
}

const MANAGED_SNIPPETS: &[&str] = &[
    "hostname.nix",
    "dns.nix",
    "users.nix",
    "custom-packages.nix",
    "hardware.nix",
    "network.nix",
    "services.nix",
    "unfree.nix",
];

/// Atomic write to a file (write to temp, then rename). Refuses paths outside
/// [`paths::MANAGED_DIR`].
pub(crate) fn atomic_write(path: &str, content: &str) -> anyhow::Result<()> {
    let path = Path::new(path);
    if !paths::is_allowed_managed_path(path) {
        anyhow::bail!(
            "Refusing to write outside {}: {}",
            paths::MANAGED_DIR,
            path.display()
        );
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = path.with_extension("tmp");
    if !paths::is_allowed_managed_path(&temp_path) {
        anyhow::bail!(
            "Refusing to write outside {}: {}",
            paths::MANAGED_DIR,
            temp_path.display()
        );
    }
    fs::write(&temp_path, content)?;
    fs::rename(&temp_path, path)?;

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

fn delete_unlisted_nix(dir: &Path, keep_stems: &[&str]) -> anyhow::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("nix") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if !keep_stems.contains(&stem) {
            fs::remove_file(&path)?;
            tracing::info!("Removed stale {}", path.display());
        }
    }
    Ok(())
}

/// Delete leftover managed `.nix` files that this apply did not generate.
pub fn cleanup_stale_managed_files(
    profiles_dir: &Path,
    bundles_dir: &Path,
    state_dir: &Path,
    keep_profile: Option<&str>,
    keep_bundles: &[String],
    keep_snippets: &[&str],
) -> anyhow::Result<()> {
    let keep_profile: Vec<&str> = keep_profile.into_iter().collect();
    delete_unlisted_nix(profiles_dir, &keep_profile)?;

    let keep_bundle_stems: Vec<&str> = keep_bundles.iter().map(String::as_str).collect();
    delete_unlisted_nix(bundles_dir, &keep_bundle_stems)?;

    if state_dir.is_dir() {
        for snippet in MANAGED_SNIPPETS {
            if keep_snippets.contains(snippet) {
                continue;
            }
            let path = state_dir.join(snippet);
            if path.is_file() {
                fs::remove_file(&path)?;
                tracing::info!("Removed stale {}", path.display());
            }
        }
    }

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
        assert!(selected.content.contains("./unfree.nix"));
        assert!(files.iter().any(|f| f.path.contains("unfree.nix")));
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

    #[test]
    fn customized_gaming_fallback_drops_steam_module_without_steam() {
        let nix = generate_fallback_bundle("gaming", &["lutris".into(), "mangohud".into()]);
        assert!(!nix.contains("programs.steam"));
        assert!(nix.contains("lutris"));
        assert!(nix.contains("mangohud"));
    }

    #[test]
    fn fallback_bundle_drops_invalid_package_tokens() {
        let nix = generate_fallback_bundle("utilities", &["htop".into(), "foo; extra".into()]);
        assert!(nix.contains("htop"));
        assert!(!nix.contains("foo; extra"));
    }

    #[test]
    fn dry_run_fail2ban_without_ssh_writes_network() {
        let files = generate_all_files(
            &None,
            &[],
            &HashMap::new(),
            None,
            &[],
            &[],
            None,
            false,
            &[],
            &NetworkConfig {
                fail2ban_enabled: true,
                ..NetworkConfig::default()
            },
            &ServicesConfig::default(),
            &HardwareConfig::default(),
            true,
        )
        .unwrap();
        let network = files
            .iter()
            .find(|f| f.path.contains("network.nix"))
            .expect("network.nix");
        assert!(network.content.contains("services.fail2ban.enable = true;"));
        assert!(!network.content.contains("services.openssh"));
        let selected = files
            .iter()
            .find(|f| f.path.contains("selected.nix"))
            .expect("selected.nix");
        assert!(selected.content.contains("./network.nix"));
    }

    #[test]
    fn is_allowed_managed_path_rejects_escape() {
        assert!(!paths::is_allowed_managed_path(Path::new(
            "../etc/nixos/nixos-toolkit/state/hostname.nix"
        )));
        assert!(!paths::is_allowed_managed_path(Path::new("/tmp/evil")));
        assert!(!paths::is_allowed_managed_path(Path::new(
            "/etc/nixos/nixos-toolkit/../../tmp/evil"
        )));
        assert!(!paths::is_allowed_managed_path(Path::new(
            "/etc/nixos/configuration.nix"
        )));
        assert!(paths::is_allowed_managed_path(Path::new(
            "/etc/nixos/nixos-toolkit/state/hostname.nix"
        )));
        assert!(paths::is_allowed_managed_path(Path::new(
            "/etc/nixos/nixos-toolkit/bundles/gaming.nix"
        )));
    }

    #[test]
    fn atomic_write_rejects_path_outside_managed_dir() {
        let err = atomic_write("/tmp/evil.nix", "nope").unwrap_err();
        assert!(err.to_string().contains("Refusing to write"));
    }

    #[test]
    fn cleanup_removes_stale_profiles_bundles_and_snippets() {
        let tmp = tempfile::tempdir().unwrap();
        let profiles = tmp.path().join("profiles");
        let bundles = tmp.path().join("bundles");
        let state = tmp.path().join("state");
        fs::create_dir_all(&profiles).unwrap();
        fs::create_dir_all(&bundles).unwrap();
        fs::create_dir_all(&state).unwrap();

        fs::write(profiles.join("gnome.nix"), "keep").unwrap();
        fs::write(profiles.join("kde.nix"), "stale").unwrap();
        fs::write(bundles.join("gaming.nix"), "keep").unwrap();
        fs::write(bundles.join("office.nix"), "stale").unwrap();
        fs::write(state.join("hostname.nix"), "stale").unwrap();
        fs::write(state.join("network.nix"), "keep").unwrap();
        fs::write(state.join("unfree.nix"), "stale").unwrap();
        fs::write(state.join("selected.nix"), "keep-selected").unwrap();
        fs::write(state.join("state.json"), "{}").unwrap();
        fs::write(state.join("notes.txt"), "not-nix").unwrap();

        cleanup_stale_managed_files(
            &profiles,
            &bundles,
            &state,
            Some("gnome"),
            &["gaming".into()],
            &["network.nix"],
        )
        .unwrap();

        assert!(profiles.join("gnome.nix").exists());
        assert!(!profiles.join("kde.nix").exists());
        assert!(bundles.join("gaming.nix").exists());
        assert!(!bundles.join("office.nix").exists());
        assert!(!state.join("hostname.nix").exists());
        assert!(state.join("network.nix").exists());
        assert!(!state.join("unfree.nix").exists());
        assert_eq!(
            fs::read_to_string(state.join("selected.nix")).unwrap(),
            "keep-selected"
        );
        assert!(state.join("state.json").exists());
        assert!(state.join("notes.txt").exists());
    }
}
