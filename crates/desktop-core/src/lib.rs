//! Coarse Linux desktop API. The library never performs privileged host writes.

use common::actions::{default_bundles, default_maintenance_actions, default_profiles, CpuArch};
use common::host;
use common::ipc::AppState;
use common::nix::{
    generate_fallback_profile, generate_preview_full_from, read_template_from, NixGenOptions,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::ffi::{c_char, CString};
use std::path::{Path, PathBuf};

const MAX_REQUEST_BYTES: usize = 1_048_576;

#[derive(Deserialize)]
#[serde(tag = "operation", content = "args", rename_all = "snake_case")]
enum Request {
    Bootstrap,
    ProbeHost,
    ParsePackages {
        input: String,
        #[serde(default)]
        existing: Vec<String>,
    },
    Validate {
        state: Box<AppState>,
    },
    Preview {
        state: Box<AppState>,
    },
    ProfilePreview {
        id: String,
    },
}

fn templates_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("NIXOS_TOOLKIT_TEMPLATES_DIR") {
        return PathBuf::from(path);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let bundled = parent.join("data/templates");
            if bundled.is_dir() {
                return bundled;
            }
        }
    }
    common::config::paths::templates_dir()
}

fn helper_path() -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("NIXOS_TOOLKIT_HELPER") {
        // An explicit override must not fall back to another helper.
        candidates.push(PathBuf::from(path));
    } else {
        candidates.extend(
            [
                "/run/current-system/sw/bin/nixos-toolkit-helper",
                "/usr/local/bin/nixos-toolkit-helper",
                "/usr/bin/nixos-toolkit-helper",
            ]
            .into_iter()
            .map(PathBuf::from),
        );
    }
    candidates
        .into_iter()
        .find(|path| {
            if !path.is_absolute() {
                return false;
            }
            if host::in_flatpak() {
                std::process::Command::new("flatpak-spawn")
                    .args(["--host", "--", "test", "-x"])
                    .arg(path)
                    .status()
                    .is_ok_and(|status| status.success())
            } else {
                use std::os::unix::fs::PermissionsExt;
                path.is_absolute()
                    && path
                        .metadata()
                        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            }
        })
        .map(|p| p.to_string_lossy().into_owned())
}

fn probe_host() -> Value {
    let system = host::detect_system();
    let helper = helper_path();
    json!({"system":system,"gpu":host::detect_gpu(),"arch":CpuArch::detect(),
        "flatpak":host::in_flatpak(),"helper_path":helper,
        "can_manage":system.is_nixos && helper.is_some()})
}

fn validate(state: &AppState) -> Result<(), String> {
    common::validate::validate_apply_fields(
        state.selected_profile.as_deref(),
        &state.enabled_bundles,
        state.hostname.as_deref(),
        &state.dns_servers,
        &state.user_groups,
        state.username.as_deref(),
        &state.custom_packages,
    )?;
    if !state.user_groups.is_empty() && state.username.is_none() {
        return Err("Enter a username before assigning groups".into());
    }
    let net = &state.network_config;
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
    let hw = &state.hardware_config;
    if hw.audio_server > 2 || hw.power_profile > 2 || hw.nvidia_driver.is_some_and(|v| v > 3) {
        return Err("Unknown hardware configuration option".into());
    }
    for (id, packages) in &state.bundle_packages {
        let catalog = default_bundles();
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

fn preview(mut state: AppState, templates: &Path) -> Result<String, String> {
    validate(&state)?;
    state.custom_packages.sort();
    state.enabled_bundles.sort();
    state.user_groups.sort();
    for packages in state.bundle_packages.values_mut() {
        packages.sort();
    }
    let profiles = default_profiles();
    let bundles = default_bundles();
    let options = NixGenOptions {
        profile: state
            .selected_profile
            .as_ref()
            .and_then(|id| profiles.iter().find(|p| &p.id == id)),
        bundles: bundles
            .iter()
            .filter(|b| state.enabled_bundles.contains(&b.id))
            .collect(),
        bundle_packages: state.bundle_packages,
        hostname: state.hostname.as_deref(),
        dns_servers: state.dns_servers,
        user_groups: state.user_groups,
        username: state.username.as_deref(),
        bluetooth_enabled: state.bluetooth_enabled,
        custom_packages: state.custom_packages,
        network_config: state.network_config,
        services_config: state.services_config,
        hardware_config: state.hardware_config,
    };
    Ok(generate_preview_full_from(&options, templates))
}

fn dispatch(request: Request) -> Result<Value, String> {
    match request {
        Request::Bootstrap => Ok(json!({"api_version":1,"profiles":default_profiles(),
            "bundles":default_bundles(),"maintenance":default_maintenance_actions(),
            "defaults":AppState::default(),"classic_snippet":host::classic_integration_snippet(),
            "flake_snippet":host::flake_integration_snippet()})),
        Request::ProbeHost => Ok(probe_host()),
        Request::ParsePackages { input, existing } => {
            let parsed = common::packages::parse_package_input(&input);
            if parsed.is_empty() && !input.trim().is_empty() {
                return Err("No valid package names found".into());
            }
            let (added, duplicates, bundled) = common::packages::classify_new_packages(
                &parsed,
                &existing.into_iter().collect::<HashSet<_>>(),
                &default_bundles(),
            );
            Ok(json!({"added":added,"duplicates":duplicates,"bundled":bundled}))
        }
        Request::Validate { state } => {
            validate(&state)?;
            Ok(json!({"valid":true}))
        }
        Request::Preview { state } => Ok(json!({"content":preview(*state,&templates_dir())?})),
        Request::ProfilePreview { id } => {
            let profiles = default_profiles();
            let profile = profiles
                .iter()
                .find(|p| p.id == id)
                .ok_or("Unknown desktop profile")?;
            let content = read_template_from(&templates_dir(), &profile.template)
                .unwrap_or_else(|_| generate_fallback_profile(&id));
            Ok(json!({"content":content}))
        }
    }
}

/// Safe, independently testable protocol entry point.
pub fn execute(input: &[u8]) -> Value {
    if input.len() > MAX_REQUEST_BYTES {
        return failure("protocol", "Request is too large");
    }
    let request = match serde_json::from_slice(input) {
        Ok(request) => request,
        Err(error) => return failure("protocol", &error.to_string()),
    };
    match dispatch(request) {
        Ok(result) => json!({"ok":true,"result":result}),
        Err(error) => failure("validation", &error),
    }
}

fn failure(kind: &str, message: &str) -> Value {
    json!({"ok":false,"error":{"kind":kind,"message":message}})
}

/// Receive a bounded UTF-8 JSON request and return an owned UTF-8 JSON C string.
///
/// # Safety
/// `input` must point to `len` readable bytes for this call. The returned pointer
/// must be released exactly once with `toolkit_free`, never by Dart's allocator.
#[no_mangle]
pub unsafe extern "C" fn toolkit_call(input: *const u8, len: usize) -> *mut c_char {
    let result = if input.is_null() || len > MAX_REQUEST_BYTES {
        failure("protocol", "Invalid request buffer")
    } else {
        std::panic::catch_unwind(|| execute(unsafe { std::slice::from_raw_parts(input, len) }))
            .unwrap_or_else(|_| failure("internal", "The core could not complete the operation"))
    };
    // JSON serialization escapes embedded NUL characters.
    CString::new(result.to_string())
        .expect("JSON has no NUL bytes")
        .into_raw()
}

/// Release a result allocated by `toolkit_call`.
///
/// # Safety
/// `result` must be null or an unreleased pointer returned by `toolkit_call`.
#[no_mangle]
pub unsafe extern "C" fn toolkit_free(result: *mut c_char) {
    if !result.is_null() {
        drop(unsafe { CString::from_raw(result) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bootstrap_has_complete_catalog_and_legacy_defaults() {
        let r = execute(br#"{"operation":"bootstrap"}"#);
        assert_eq!(r["result"]["profiles"].as_array().unwrap().len(), 13);
        assert_eq!(r["result"]["bundles"].as_array().unwrap().len(), 16);
        assert_eq!(r["result"]["defaults"]["network_config"]["ssh_port"], 22);
    }
    #[test]
    fn malformed_request_and_inputs_return_typed_failures() {
        assert_eq!(execute(b"invalid")["error"]["kind"], "protocol");
        let r = execute(br#"{"operation":"parse_packages","args":{"input":"] ["}}"#);
        assert_eq!(r["error"]["kind"], "validation");
    }
    #[test]
    fn package_classification_preserves_catalog_duplicates() {
        let r=execute(br#"{"operation":"parse_packages","args":{"input":"git, ripgrep pkgs.fd","existing":["fd"]}}"#);
        assert_eq!(r["result"]["added"], json!(["ripgrep"]));
        assert_eq!(r["result"]["duplicates"], json!(["fd"]));
        assert_eq!(r["result"]["bundled"][0][0], "git");
    }
    #[test]
    fn preview_contains_custom_package_and_missing_profile_fallback() {
        let state = AppState {
            selected_profile: Some("gnome".into()),
            custom_packages: vec!["ripgrep".into()],
            ..Default::default()
        };
        let text = preview(state, Path::new("/nonexistent-templates")).unwrap();
        assert!(text.contains("services.desktopManager.gnome.enable = true"));
        assert!(text.contains("ripgrep"));
    }
    #[test]
    fn ffi_roundtrip_owns_and_frees_results() {
        let input = br#"{"operation":"bootstrap"}"#;
        for _ in 0..100 {
            let output = unsafe { toolkit_call(input.as_ptr(), input.len()) };
            let bytes = unsafe { std::ffi::CStr::from_ptr(output) }.to_bytes();
            assert_eq!(serde_json::from_slice::<Value>(bytes).unwrap()["ok"], true);
            unsafe { toolkit_free(output) };
        }
        let output = unsafe { toolkit_call(std::ptr::null(), 0) };
        assert!(!output.is_null());
        unsafe { toolkit_free(output) };
    }
}
