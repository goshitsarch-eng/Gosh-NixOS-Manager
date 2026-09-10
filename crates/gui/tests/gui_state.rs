//! Apply-empty classification and dialog copies.
//!
//! Exhaustive `Message` match lives in `crates/gui/src/core/apply.rs` and is not
//! duplicated here. `AppModel::apply` is exercised in that module because
//! `dialog` is `pub(crate)` and there is no display-free `AppModel::test_model()`.
//!
//! Empty-apply is profile + bundles + custom packages, plus non-default
//! hardware after C3/N5. Network/services still do not count. PackagesOnly vs
//! Normal is not distinct.

use nixos_toolkit_gui::{AppState, Dialog, Flags};

#[test]
fn flags_for_tests_skip_privileged_init() {
    let flags = Flags::for_tests();
    assert!(flags.skip_privileged_on_init);
    assert!(flags.skip_host_probes);
    assert!(!flags.spawn.helper_available);
}

#[test]
fn empty_state_is_empty_apply() {
    assert!(AppState::new().apply_is_empty());
}

#[test]
fn network_or_services_only_still_count_as_empty_apply() {
    let mut state = AppState::new();
    state.network_config.ssh_enabled = true;
    state.services_config.printing = true;
    assert!(state.apply_is_empty());
}

#[test]
fn non_default_hardware_is_not_empty_apply() {
    let mut with_nvidia = AppState::new();
    with_nvidia.hardware_config.nvidia_driver = Some(0);
    assert!(!with_nvidia.apply_is_empty());

    let mut with_bluetooth = AppState::new();
    with_bluetooth.set_bluetooth_enabled(true);
    assert!(!with_bluetooth.apply_is_empty());
}

#[test]
fn profile_bundles_or_packages_are_not_empty_apply() {
    let mut with_profile = AppState::new();
    with_profile.select_profile("gnome");
    assert!(!with_profile.apply_is_empty());

    let mut with_bundle = AppState::new();
    with_bundle.enable_bundle("devtools");
    assert!(!with_bundle.apply_is_empty());

    let mut with_pkg = AppState::new();
    with_pkg.add_custom_package("htop");
    assert!(!with_pkg.apply_is_empty());
}

#[test]
fn packages_only_is_not_distinct_from_normal_in_current_bool() {
    let mut packages_only = AppState::new();
    packages_only.add_custom_package("htop");
    let mut normal = AppState::new();
    normal.select_profile("gnome");
    assert_eq!(packages_only.apply_is_empty(), normal.apply_is_empty());
    assert!(!packages_only.apply_is_empty());
}

#[test]
fn confirm_apply_dialog_empty_vs_not() {
    let empty = Dialog::ConfirmApply {
        heading: "Apply Empty Configuration?".into(),
        body: "This will remove ALL managed software from the generated NixOS configuration."
            .into(),
        destructive: true,
    };
    let not_empty = Dialog::ConfirmApply {
        heading: "Apply Configuration?".into(),
        body: "This will run nixos-rebuild switch.".into(),
        destructive: false,
    };
    assert_ne!(empty, not_empty);
    match empty {
        Dialog::ConfirmApply { destructive, .. } => assert!(destructive),
        other => panic!("expected ConfirmApply, got {other:?}"),
    }
    match not_empty {
        Dialog::ConfirmApply { destructive, .. } => assert!(!destructive),
        other => panic!("expected ConfirmApply, got {other:?}"),
    }
}
