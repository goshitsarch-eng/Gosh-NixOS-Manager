//! Shared validation for values that become Nix or helper IPC payloads.

use crate::actions::{default_bundles, default_profiles};
use crate::ipc::{AppState, HardwareConfig, NetworkConfig};
use crate::nix::is_nix_attrpath;
use std::collections::HashMap;

/// Maximum hostname length (DNS label).
pub const MAX_HOSTNAME_LEN: usize = 63;

/// Maximum JSON-line IPC payload the helper will accept.
pub const MAX_HELPER_JSON_LINE: usize = 1_048_576;

/// GUI/helper hostname charset: ASCII alphanumerics and `-`, max 63.
#[must_use]
pub fn hostname_is_valid(hostname: &str) -> bool {
    !hostname.is_empty()
        && hostname.len() <= MAX_HOSTNAME_LEN
        && hostname
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && hostname
            .as_bytes()
            .last()
            .is_some_and(u8::is_ascii_alphanumeric)
        && hostname
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Unix-ish username: ASCII alphanumerics, `_`, `-`.
#[must_use]
pub fn username_is_valid(username: &str) -> bool {
    !username.is_empty()
        && username.len() <= 32
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Group names written into `users.users.<name>.extraGroups`.
#[must_use]
pub fn group_name_is_valid(group: &str) -> bool {
    !group.is_empty()
        && group.len() <= 32
        && group
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Strict dotted IPv4 (no leading zeros, 4 octets).
#[must_use]
pub fn is_ipv4(s: &str) -> bool {
    let mut parts = s.split('.');
    let mut count = 0;
    for part in parts.by_ref() {
        count += 1;
        if count > 4 {
            return false;
        }
        if part.is_empty() || part.len() > 3 {
            return false;
        }
        if !part.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        if part.len() > 1 && part.starts_with('0') {
            return false;
        }
        if part.parse::<u8>().is_err() {
            return false;
        }
    }
    count == 4
}

/// Reject unknown profiles and strings that cannot be interpolated safely.
pub fn validate_apply_fields(
    selected_profile: Option<&str>,
    enabled_bundles: &[String],
    hostname: Option<&str>,
    dns_servers: &[String],
    user_groups: &[String],
    username: Option<&str>,
    custom_packages: &[String],
) -> Result<(), String> {
    if let Some(profile) = selected_profile {
        if !default_profiles().iter().any(|p| p.id == profile) {
            return Err(format!("Unknown profile: {profile}"));
        }
    }

    let valid_bundles: Vec<String> = default_bundles().into_iter().map(|b| b.id).collect();
    for bundle in enabled_bundles {
        if !valid_bundles.iter().any(|id| id == bundle) {
            return Err(format!("Unknown bundle: {bundle}"));
        }
    }

    if let Some(host) = hostname {
        if !hostname_is_valid(host) {
            return Err(format!("Invalid hostname: {host}"));
        }
    }

    for dns in dns_servers {
        if !is_ipv4(dns) {
            return Err(format!("Invalid DNS server: {dns}"));
        }
    }

    if let Some(user) = username {
        if !username_is_valid(user) {
            return Err(format!("Invalid username: {user}"));
        }
    }

    for group in user_groups {
        if !group_name_is_valid(group) {
            return Err(format!("Invalid group name: {group}"));
        }
    }

    if !user_groups.is_empty() && username.is_none() {
        return Err("Enter a username before assigning groups".into());
    }

    for pkg in custom_packages {
        if !is_nix_attrpath(pkg) {
            return Err(format!("Invalid package attribute: {pkg}"));
        }
    }

    Ok(())
}

/// Validate the structured selections at both the preview and privileged boundary.
pub fn validate_configuration_options(
    bundle_packages: &HashMap<String, Vec<String>>,
    net: &NetworkConfig,
    hw: &HardwareConfig,
) -> Result<(), String> {
    if net.ssh_port == 0
        || net.wireguard_listen_port == 0
        || net.allowed_tcp_ports.contains(&0)
        || net.allowed_udp_ports.contains(&0)
    {
        return Err("Ports must be between 1 and 65535".into());
    }
    if !["no", "prohibit-password", "yes"].contains(&net.ssh_root_login.as_str()) {
        return Err("Unknown SSH root login policy".into());
    }
    if hw.audio_server > 2 || hw.power_profile > 2 || hw.nvidia_driver.is_some_and(|v| v > 3) {
        return Err("Unknown hardware configuration option".into());
    }
    let catalog = default_bundles();
    for (id, packages) in bundle_packages {
        let bundle = catalog
            .iter()
            .find(|b| &b.id == id)
            .ok_or("Unknown bundle package selection")?;
        if packages
            .iter()
            .any(|p| !bundle.packages.iter().any(|def| &def.id == p))
        {
            return Err(format!("Unknown package selected in bundle {id}"));
        }
    }
    Ok(())
}

pub fn validate_state(state: &AppState) -> Result<(), String> {
    validate_apply_fields(
        state.selected_profile.as_deref(),
        &state.enabled_bundles,
        state.hostname.as_deref(),
        &state.dns_servers,
        &state.user_groups,
        state.username.as_deref(),
        &state.custom_packages,
    )?;
    validate_configuration_options(
        &state.bundle_packages,
        &state.network_config,
        &state.hardware_config,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostname_rejects_newline_and_unicode() {
        assert!(hostname_is_valid("nixos"));
        assert!(!hostname_is_valid(""));
        assert!(!hostname_is_valid("foo.bar"));
        assert!(!hostname_is_valid("höst"));
        assert!(!hostname_is_valid("a\nnixos"));
        assert!(!hostname_is_valid(&"a".repeat(64)));
    }

    #[test]
    fn ipv4_parser() {
        assert!(is_ipv4("1.1.1.1"));
        assert!(is_ipv4("255.255.255.255"));
        assert!(!is_ipv4("1.1.1"));
        assert!(!is_ipv4("1.1.1.1.1"));
        assert!(!is_ipv4("01.1.1.1"));
        assert!(!is_ipv4("1.1.1.256"));
        assert!(!is_ipv4("::1"));
        assert!(!is_ipv4("1.1.1.1\n};"));
    }

    #[test]
    fn hostname_requires_alphanumeric_edges() {
        for invalid in ["-", "---", "-desktop", "desktop-"] {
            assert!(!hostname_is_valid(invalid));
        }
        for valid in ["a", "desk-1", "42"] {
            assert!(hostname_is_valid(valid));
        }
    }

    #[test]
    fn apply_fields_reject_comment_breakout() {
        let err = validate_apply_fields(None, &[], None, &["1.1.1.1\n};".into()], &[], None, &[])
            .unwrap_err();
        assert!(err.contains("DNS"));
    }

    #[test]
    fn apply_fields_accept_catalog() {
        assert!(validate_apply_fields(
            Some("gnome"),
            &["gaming".into()],
            Some("desktop"),
            &["1.1.1.1".into()],
            &["docker".into()],
            Some("alice"),
            &["htop".into()],
        )
        .is_ok());
    }

    #[test]
    fn apply_fields_reject_unknown_profile() {
        assert!(validate_apply_fields(Some("../etc"), &[], None, &[], &[], None, &[]).is_err());
    }

    #[test]
    fn package_attributes_require_valid_nonempty_segments() {
        for invalid in [
            "hello.",
            "python3Packages.0",
            "foo..bar",
            ".hello",
            "foo.-bar",
        ] {
            assert!(
                validate_apply_fields(None, &[], None, &[], &[], None, &[invalid.into()]).is_err(),
                "{invalid}"
            );
        }
        assert!(is_nix_attrpath("python3Packages.numpy"));
        assert!(is_nix_attrpath("nerd-fonts.fira-code"));
    }
}
