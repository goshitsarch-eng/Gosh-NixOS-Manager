//! Apply-empty classification and dialog copies.
//!
//! Exhaustive `Message` match lives in `crates/gui/src/core/apply.rs` and is not
//! duplicated here. `AppModel::apply` is exercised in that module because
//! `dialog` is `pub(crate)` and there is no display-free `AppModel::test_model()`.

use nixos_toolkit_gui::{AppState, Dialog, Flags};

/// Current architecture empty-apply predicate (profile + bundles + custom packages).
/// Network/services/hardware do not count; PackagesOnly vs Normal is not distinct.
fn apply_is_empty(state: &AppState) -> bool {
    state.selected_profile.is_none()
        && state.enabled_bundles.is_empty()
        && state.custom_packages.is_empty()
}

#[test]
fn flags_for_tests_skip_privileged_init() {
    let flags = Flags::for_tests();
    assert!(flags.skip_privileged_on_init);
    assert!(flags.skip_host_probes);
    assert!(!flags.spawn.helper_available);
}

#[test]
fn empty_state_is_empty_apply() {
    assert!(apply_is_empty(&AppState::new()));
}

#[test]
fn network_or_services_only_still_count_as_empty_apply() {
    let mut state = AppState::new();
    state.network_config.ssh_enabled = true;
    state.services_config.printing = true;
    state.set_bluetooth_enabled(true);
    assert!(apply_is_empty(&state));
}

#[test]
fn profile_bundles_or_packages_are_not_empty_apply() {
    let mut with_profile = AppState::new();
    with_profile.select_profile("gnome");
    assert!(!apply_is_empty(&with_profile));

    let mut with_bundle = AppState::new();
    with_bundle.enable_bundle("devtools");
    assert!(!apply_is_empty(&with_bundle));

    let mut with_pkg = AppState::new();
    with_pkg.add_custom_package("htop");
    assert!(!apply_is_empty(&with_pkg));
}

#[test]
fn packages_only_is_not_distinct_from_normal_in_current_bool() {
    let mut packages_only = AppState::new();
    packages_only.add_custom_package("htop");
    let mut normal = AppState::new();
    normal.select_profile("gnome");
    assert_eq!(apply_is_empty(&packages_only), apply_is_empty(&normal));
    assert!(!apply_is_empty(&packages_only));
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
