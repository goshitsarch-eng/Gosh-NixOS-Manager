//! Toolkit-independent package input parsing and catalog classification.

use crate::actions::BundleDef;
use std::collections::{HashMap, HashSet};

/// Parse user input — supports Nix expressions, commas, newlines, and spaces.
#[must_use]
pub fn parse_package_input(input: &str) -> Vec<String> {
    let input = input.trim();
    let content = if let Some(start) = input.find('[') {
        match input.rfind(']') {
            Some(end) if end > start && !input[..start].contains(']') => &input[start + 1..end],
            _ => return Vec::new(),
        }
    } else {
        input.strip_prefix("with pkgs;").unwrap_or(input)
    };
    content
        .split(|c: char| c.is_whitespace() || c == ',')
        .map(clean_package_name)
        .filter(|name| is_valid_package_name(name))
        .collect()
}

/// Clean a package name — remove prefixes and suffixes.
#[must_use]
pub fn clean_package_name(name: &str) -> String {
    let mut name = name.trim();

    // Strip pkgs. prefix
    if name.starts_with("pkgs.") {
        name = &name[5..];
    }

    // Strip nixpkgs# prefix (for flake references)
    if name.starts_with("nixpkgs#") {
        name = &name[8..];
    }

    // Strip trailing semicolons and brackets
    name = name
        .trim_end_matches(';')
        .trim_end_matches(']')
        .trim_start_matches('[');

    name.trim().to_string()
}

/// Check if a package name is a valid nixpkgs identifier.
#[must_use]
pub fn is_valid_package_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 128 {
        return false;
    }

    crate::nix::is_nix_attrpath(name)
}

/// Map every catalog package id to its bundle **display name**.
///
/// Duplicate detection uses **all** bundles, not only enabled ones (GTK).
#[must_use]
pub fn get_all_bundle_packages(bundles: &[BundleDef]) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for bundle in bundles {
        for pkg in &bundle.packages {
            map.insert(pkg.id.clone(), bundle.name.clone());
        }
    }

    map
}

/// Classify requested names against already-custom packages and the catalog.
///
/// Returns `(added, already_custom, in_bundle)` where `in_bundle` is
/// `(package_id, bundle_display_name)`.
#[must_use]
pub fn classify_new_packages(
    requested: &[String],
    existing: &HashSet<String>,
    bundles: &[BundleDef],
) -> (Vec<String>, Vec<String>, Vec<(String, String)>) {
    let mut added = Vec::new();
    let mut already_custom = Vec::new();
    let mut in_bundle = Vec::new();

    let bundle_packages = get_all_bundle_packages(bundles);
    let mut current = existing.clone();

    for pkg in requested {
        if current.contains(pkg) {
            already_custom.push(pkg.clone());
        } else if let Some(bundle_name) = bundle_packages.get(pkg) {
            in_bundle.push((pkg.clone(), bundle_name.clone()));
        } else {
            current.insert(pkg.clone());
            added.push(pkg.clone());
        }
    }

    (added, already_custom, in_bundle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{default_bundles, BundleDef, PackageDef};

    fn bundles_with(id: &str, name: &str, packages: &[&str]) -> Vec<BundleDef> {
        let mut bundles = default_bundles();
        bundles.push(BundleDef {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            icon: String::new(),
            category: crate::actions::ActionCategory::System,
            template: String::new(),
            packages: packages.iter().map(|p| PackageDef::new(*p, *p)).collect(),
            arm_compat: crate::actions::ArmCompat::Full,
            arm_note: None,
        });
        bundles
    }

    #[test]
    fn empty_and_whitespace_yield_nothing() {
        assert!(parse_package_input("").is_empty());
        assert!(parse_package_input("   ").is_empty());
    }

    #[test]
    fn single_token() {
        assert_eq!(parse_package_input("neofetch"), vec!["neofetch"]);
        assert_eq!(parse_package_input("  ripgrep  "), vec!["ripgrep"]);
    }

    #[test]
    fn mixed_separators_and_malformed_brackets() {
        assert_eq!(
            parse_package_input("alpha, beta gamma\ndelta"),
            vec!["alpha", "beta", "gamma", "delta"]
        );
        assert!(parse_package_input("] [").is_empty());
        assert!(parse_package_input("[ foo").is_empty());
    }

    #[test]
    fn comma_separated() {
        assert_eq!(
            parse_package_input("neofetch, ripgrep, fd"),
            vec!["neofetch", "ripgrep", "fd"]
        );
    }

    #[test]
    fn newline_separated() {
        assert_eq!(
            parse_package_input("neofetch\nripgrep\nfd"),
            vec!["neofetch", "ripgrep", "fd"]
        );
    }

    #[test]
    fn space_separated() {
        assert_eq!(
            parse_package_input("neofetch ripgrep fd"),
            vec!["neofetch", "ripgrep", "fd"]
        );
    }

    #[test]
    fn pkgs_prefixed_single() {
        assert_eq!(parse_package_input("pkgs.neofetch"), vec!["neofetch"]);
    }

    #[test]
    fn pkgs_prefixed_space_separated() {
        assert_eq!(parse_package_input("pkgs.foo pkgs.bar"), vec!["foo", "bar"]);
    }

    #[test]
    fn nixpkgs_hash_prefix() {
        assert_eq!(parse_package_input("nixpkgs#ripgrep"), vec!["ripgrep"]);
    }

    #[test]
    fn environment_system_packages_brackets() {
        assert_eq!(
            parse_package_input("environment.systemPackages = [ pkgs.foo pkgs.bar ];"),
            vec!["foo", "bar"]
        );
    }

    #[test]
    fn with_pkgs_then_names() {
        assert_eq!(
            parse_package_input("with pkgs; neofetch ripgrep"),
            vec!["neofetch", "ripgrep"]
        );
    }

    #[test]
    fn with_pkgs_bracket_list() {
        assert_eq!(
            parse_package_input("with pkgs; [ neofetch ripgrep ];"),
            vec!["neofetch", "ripgrep"]
        );
    }

    #[test]
    fn strips_trailing_semicolon_and_brackets() {
        assert_eq!(clean_package_name("pkgs.foo;"), "foo");
        assert_eq!(clean_package_name("[bar]"), "bar");
        assert_eq!(clean_package_name("nixpkgs#baz;"), "baz");
    }

    #[test]
    fn valid_names() {
        assert!(is_valid_package_name("a"));
        assert!(is_valid_package_name("zed-editor"));
        assert!(is_valid_package_name("python3Packages.pip"));
        assert!(is_valid_package_name("nerd-fonts.fira-code"));
        assert!(is_valid_package_name(&"a".repeat(128)));
        assert!(is_valid_package_name("_1password-gui"));
        assert!(is_valid_package_name("_hidden"));
    }

    #[test]
    fn invalid_names() {
        assert!(!is_valid_package_name(""));
        assert!(!is_valid_package_name("1password"));
        assert!(!is_valid_package_name("foo bar"));
        assert!(!is_valid_package_name("foo/bar"));
        assert!(!is_valid_package_name(&"a".repeat(129)));
        assert!(parse_package_input("123abc").is_empty());
        assert_eq!(
            parse_package_input("_1password-gui"),
            vec!["_1password-gui"]
        );
    }

    #[test]
    fn classify_adds_unknown_packages() {
        let existing = HashSet::new();
        let (added, dup, bundled) =
            classify_new_packages(&["neofetch".into()], &existing, &default_bundles());
        assert_eq!(added, vec!["neofetch"]);
        assert!(dup.is_empty());
        assert!(bundled.is_empty());
    }

    #[test]
    fn classify_reports_already_custom() {
        let existing = HashSet::from(["neofetch".into()]);
        let (added, dup, bundled) =
            classify_new_packages(&["neofetch".into()], &existing, &default_bundles());
        assert!(added.is_empty());
        assert_eq!(dup, vec!["neofetch"]);
        assert!(bundled.is_empty());
    }

    #[test]
    fn classify_duplicate_within_request() {
        let existing = HashSet::new();
        let (added, dup, _) = classify_new_packages(
            &["neofetch".into(), "neofetch".into()],
            &existing,
            &default_bundles(),
        );
        assert_eq!(added, vec!["neofetch"]);
        assert_eq!(dup, vec!["neofetch"]);
    }

    #[test]
    fn classify_in_bundle_uses_all_bundles_not_only_enabled() {
        // `git` lives in Development Tools. GTK checked every catalog bundle.
        let existing = HashSet::new();
        let (added, dup, bundled) =
            classify_new_packages(&["git".into()], &existing, &default_bundles());
        assert!(added.is_empty());
        assert!(dup.is_empty());
        assert_eq!(bundled.len(), 1);
        assert_eq!(bundled[0].0, "git");
        assert_eq!(bundled[0].1, "Development Tools");
    }

    #[test]
    fn classify_in_bundle_uses_display_name() {
        let bundles = bundles_with("custom", "Custom Bundle Name", &["unique-pkg-xyz"]);
        let existing = HashSet::new();
        let (_, _, bundled) =
            classify_new_packages(&["unique-pkg-xyz".into()], &existing, &bundles);
        assert_eq!(
            bundled,
            vec![("unique-pkg-xyz".into(), "Custom Bundle Name".into())]
        );
    }

    #[test]
    fn classify_existing_custom_wins_over_bundle() {
        let existing = HashSet::from(["git".into()]);
        let (_, dup, bundled) =
            classify_new_packages(&["git".into()], &existing, &default_bundles());
        assert_eq!(dup, vec!["git"]);
        assert!(bundled.is_empty());
    }
}
