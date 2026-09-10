//! Public `core::packages` API. Case-by-case parser tests live in
//! `crates/gui/src/core/packages.rs` and are not duplicated here.

use nixos_toolkit_gui::core::packages::parse_package_input;

#[test]
fn parser_is_available_to_integration_tests() {
    assert_eq!(
        parse_package_input("neofetch, ripgrep"),
        vec!["neofetch", "ripgrep"]
    );
    assert!(parse_package_input("123invalid").is_empty());
}
