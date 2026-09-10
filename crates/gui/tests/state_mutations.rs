//! AppState public mutations. These tests do not construct [`nixos_toolkit_gui::AppModel`]
//! (that still needs `cosmic::Core` / `Application::init`; see TASKLOG).

use nixos_toolkit_gui::AppState;
use std::collections::HashSet;

#[test]
fn select_and_clear_profile() {
    let mut state = AppState::new();
    state.select_profile("gnome");
    assert_eq!(state.selected_profile.as_deref(), Some("gnome"));
    assert!(state.has_changes);
    state.mark_applied();
    assert!(!state.has_changes);

    state.clear_profile();
    assert!(state.selected_profile.is_none());
    assert!(state.has_changes);
}

#[test]
fn toggle_enable_disable_bundles_and_packages() {
    let mut state = AppState::new();
    state.toggle_bundle("devtools");
    assert!(state.is_bundle_enabled("devtools"));
    state.toggle_bundle("devtools");
    assert!(!state.is_bundle_enabled("devtools"));

    state.enable_bundle("gaming");
    let pkgs: HashSet<String> = ["steam", "lutris"]
        .into_iter()
        .map(str::to_string)
        .collect();
    state.set_bundle_packages("gaming", pkgs.clone());
    assert_eq!(state.get_bundle_packages("gaming"), Some(&pkgs));

    state.toggle_bundle_package("gaming", "steam", false);
    assert!(!state
        .get_bundle_packages("gaming")
        .expect("gaming packages")
        .contains("steam"));

    state.disable_bundle("gaming");
    assert!(!state.is_bundle_enabled("gaming"));
    assert!(state.get_bundle_packages("gaming").is_none());
}

#[test]
fn toggle_bundle_package_is_noop_until_set_exists() {
    let mut state = AppState::new();
    state.toggle_bundle_package("missing", "git", true);
    assert!(state.get_bundle_packages("missing").is_none());
}

#[test]
fn bluetooth_mirror_writes_both_and_prefers_hardware_on_diverge() {
    let mut state = AppState::new();
    state.set_bluetooth_enabled(true);
    assert!(state.bluetooth_enabled);
    assert!(state.hardware_config.bluetooth_enabled);

    state.hardware_config.bluetooth_enabled = false;
    state.sync_bluetooth_from_hardware();
    assert!(!state.bluetooth_enabled);

    let mut ipc = state.to_ipc_state();
    ipc.bluetooth_enabled = false;
    ipc.hardware_config.bluetooth_enabled = true;
    let restored = AppState::from_ipc_state(ipc);
    assert!(restored.bluetooth_enabled);
    assert!(restored.hardware_config.bluetooth_enabled);

    let mut hardware = restored.hardware_config.clone();
    hardware.bluetooth_enabled = false;
    let mut state = restored;
    state.set_hardware_config(hardware);
    assert!(!state.bluetooth_enabled);
    assert!(!state.hardware_config.bluetooth_enabled);
}

#[test]
fn tcp_ports_split_parse_and_toggle() {
    let mut state = AppState::new();
    state.set_tcp_port(22, true);
    state
        .parse_and_add_tcp_ports("80, 443\n8080 80")
        .expect("ports");
    assert_eq!(
        state.network_config.allowed_tcp_ports,
        vec![22, 80, 443, 8080]
    );

    state.set_tcp_port(22, false);
    assert_eq!(state.network_config.allowed_tcp_ports, vec![80, 443, 8080]);

    assert!(state.parse_and_add_tcp_ports("0").is_err());
    assert!(state.parse_and_add_tcp_ports("not-a-port").is_err());
    assert_eq!(state.network_config.allowed_tcp_ports, vec![80, 443, 8080]);
}

#[test]
fn custom_udp_replace_keeps_presets_and_drops_prefixes() {
    let mut state = AppState::new();
    state.set_udp_port(53, true);
    state.set_udp_port(443, true);
    state
        .parse_and_set_custom_udp_ports("9")
        .expect("prefix port");
    state
        .parse_and_set_custom_udp_ports("90")
        .expect("prefix port");
    state
        .parse_and_set_custom_udp_ports("9090")
        .expect("custom port");
    assert_eq!(state.network_config.allowed_udp_ports, vec![53, 443, 9090]);

    state
        .parse_and_set_custom_udp_ports("123, 9091")
        .expect("preset in extras ignored");
    assert_eq!(state.network_config.allowed_udp_ports, vec![53, 443, 9091]);

    assert!(state.parse_and_set_custom_udp_ports("abc").is_err());
    assert_eq!(state.network_config.allowed_udp_ports, vec![53, 443, 9091]);
}

#[test]
fn custom_tcp_replace_keeps_presets_and_drops_prefixes() {
    let mut state = AppState::new();
    state.set_tcp_port(22, true);
    state.set_tcp_port(443, true);
    state
        .parse_and_set_custom_tcp_ports("9")
        .expect("prefix port");
    state
        .parse_and_set_custom_tcp_ports("90")
        .expect("prefix port");
    state
        .parse_and_set_custom_tcp_ports("9090")
        .expect("custom port");
    assert_eq!(state.network_config.allowed_tcp_ports, vec![22, 443, 9090]);

    state
        .parse_and_set_custom_tcp_ports("80, 9091")
        .expect("preset in extras ignored");
    assert_eq!(state.network_config.allowed_tcp_ports, vec![22, 443, 9091]);

    assert!(state.parse_and_set_custom_tcp_ports("abc").is_err());
    assert_eq!(state.network_config.allowed_tcp_ports, vec![22, 443, 9091]);
}

#[test]
fn udp_ports_survive_ipc_and_json_round_trip() {
    let mut state = AppState::new();
    state.network_config.allowed_udp_ports = vec![53, 123, 51820];
    state.set_tcp_port(22, true);

    let restored = AppState::from_ipc_state(state.to_ipc_state());
    assert_eq!(
        restored.network_config.allowed_udp_ports,
        vec![53, 123, 51820]
    );
    assert_eq!(restored.network_config.allowed_tcp_ports, vec![22]);

    let json = serde_json::to_string(&state.to_ipc_state()).expect("serialize ipc state");
    assert!(json.contains("allowed_udp_ports"));
    let restored = AppState::from_ipc_state(serde_json::from_str(&json).expect("deserialize"));
    assert_eq!(
        restored.network_config.allowed_udp_ports,
        vec![53, 123, 51820]
    );
    assert_eq!(restored.network_config.allowed_tcp_ports, vec![22]);
}

#[test]
fn adding_tcp_does_not_drop_udp() {
    let mut state = AppState::new();
    state.network_config.allowed_udp_ports = vec![53];
    state.parse_and_add_tcp_ports("22").expect("tcp");
    assert_eq!(state.network_config.allowed_udp_ports, vec![53]);
    assert_eq!(state.network_config.allowed_tcp_ports, vec![22]);
}

#[test]
fn services_set_known_ids() {
    let mut state = AppState::new();
    state.set_service("printing", true);
    state.set_service("docker", true);
    state.set_service("auto_gc", true);
    assert!(state.services_config.printing);
    assert!(state.services_config.docker);
    assert!(state.services_config.auto_gc);
    assert!(!state.services_config.avahi);

    state.set_service("printing", false);
    assert!(!state.services_config.printing);
    assert!(state.has_changes);

    let before = state.services_config.clone();
    state.set_service("not-a-service", true);
    assert_eq!(state.services_config.printing, before.printing);
    assert_eq!(state.services_config.docker, before.docker);
}

#[test]
fn hostname_dns_groups_and_custom_packages() {
    let mut state = AppState::new();
    state.set_hostname("nixos");
    state.parse_and_set_dns("1.1.1.1, 8.8.8.8").expect("dns");
    assert!(state.parse_and_set_dns("not-an-ip").is_err());
    state.set_username("gosh");
    state.set_user_group("wheel", true);
    state.add_custom_package("htop");

    assert_eq!(state.hostname.as_deref(), Some("nixos"));
    assert_eq!(state.dns_servers, ["1.1.1.1", "8.8.8.8"]);
    assert_eq!(state.username.as_deref(), Some("gosh"));
    assert!(state.is_in_group("wheel"));
    assert!(state.has_custom_package("htop"));

    state.remove_custom_package("htop");
    assert!(!state.has_custom_package("htop"));
}
