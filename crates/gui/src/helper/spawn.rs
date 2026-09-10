//! Injectable helper spawn specification (DECISIONS C9 / C10).

use std::path::{Path, PathBuf};
use std::process::Command;

const HELPER_NAME: &str = "nixos-toolkit-helper";

const SYSTEM_HELPER_PATHS: &[&str] = &[
    "/run/current-system/sw/bin/nixos-toolkit-helper",
    "/usr/local/bin/nixos-toolkit-helper",
    "/usr/bin/nixos-toolkit-helper",
];

/// How the GUI launches `nixos-toolkit-helper`.
#[derive(Debug, Clone)]
pub struct SpawnSpec {
    /// e.g. `pkexec`, `flatpak-spawn`, or the helper path itself.
    pub program: PathBuf,
    /// Arguments before helper stdin/stdout are attached.
    pub args: Vec<String>,
    pub extra_env: Vec<(String, String)>,
    /// Always includes `SHELL` unless tests say otherwise.
    pub remove_env: Vec<String>,
    pub templates_dir: Option<PathBuf>,
    /// False when the Flatpak host probe found no helper — do not pkexec.
    pub helper_available: bool,
}

/// Inputs for [`SpawnSpec::from_parts`] (env-free, testable).
#[derive(Debug, Clone)]
pub struct SpawnInputs {
    pub no_pkexec: bool,
    pub spawn_program: Option<String>,
    pub spawn_args: Vec<String>,
    pub helper_env: Option<String>,
    pub templates_dir: Option<PathBuf>,
    pub in_flatpak: bool,
    /// Host helper path found via `flatpak-spawn --host -- test -x`.
    pub probed_helper: Option<String>,
    /// Host install discovery result (may use `Path::exists`).
    pub host_helper: String,
    /// Host helper path is an executable file (or `which` found it).
    pub host_helper_executable: bool,
}

impl SpawnSpec {
    /// Build from process environment. Never `Path::exists` on host helper
    /// paths when running inside Flatpak.
    #[must_use]
    pub fn from_env() -> Self {
        let in_flatpak = in_flatpak();
        let helper_env = nonempty_env("NIXOS_TOOLKIT_HELPER");
        let templates_dir = nonempty_env("NIXOS_TOOLKIT_TEMPLATES_DIR").map(PathBuf::from);
        let no_pkexec = env_flag("NIXOS_TOOLKIT_NO_PKEXEC");
        let spawn_program = nonempty_env("NIXOS_TOOLKIT_SPAWN");
        let spawn_args = match nonempty_env("NIXOS_TOOLKIT_SPAWN_ARGS") {
            Some(raw) => parse_spawn_args(&raw),
            None => Vec::new(),
        };

        let probed_helper = if in_flatpak && !no_pkexec && spawn_program.is_none() {
            probe_flatpak_helper(helper_env.as_deref())
        } else {
            None
        };

        let host_helper = if in_flatpak {
            HELPER_NAME.to_string()
        } else {
            discover_helper_path(helper_env.as_deref())
        };
        let resolved_helper = helper_env.clone().unwrap_or_else(|| host_helper.clone());
        let host_helper_executable = helper_is_executable(&resolved_helper);

        Self::from_parts(SpawnInputs {
            no_pkexec,
            spawn_program,
            spawn_args,
            helper_env,
            templates_dir,
            in_flatpak,
            probed_helper,
            host_helper,
            host_helper_executable,
        })
    }

    /// Pure construction used by [`Self::from_env`] and unit tests.
    #[must_use]
    pub fn from_parts(inputs: SpawnInputs) -> Self {
        let templates_dir = inputs.templates_dir.clone();
        let extra_env = templates_dir
            .as_ref()
            .map(|p| {
                vec![(
                    "NIXOS_TOOLKIT_TEMPLATES_DIR".to_string(),
                    p.display().to_string(),
                )]
            })
            .unwrap_or_default();
        let remove_env = vec!["SHELL".to_string()];

        if inputs.no_pkexec {
            let helper = inputs
                .helper_env
                .or(inputs.probed_helper)
                .unwrap_or(inputs.host_helper);
            return Self {
                program: PathBuf::from(helper),
                args: Vec::new(),
                extra_env,
                remove_env,
                templates_dir,
                helper_available: true,
            };
        }

        if let Some(spawn) = inputs.spawn_program {
            let helper = inputs
                .helper_env
                .or(inputs.probed_helper)
                .unwrap_or(inputs.host_helper);
            let mut args = inputs.spawn_args;
            args.push(helper);
            return Self {
                program: PathBuf::from(spawn),
                args,
                extra_env,
                remove_env,
                templates_dir,
                helper_available: true,
            };
        }

        if inputs.in_flatpak {
            if let Some(path) = inputs.probed_helper {
                return Self {
                    program: PathBuf::from("flatpak-spawn"),
                    args: vec![
                        "--host".into(),
                        "--forward-fd=0".into(),
                        "--forward-fd=1".into(),
                        "--".into(),
                        "pkexec".into(),
                        path,
                    ],
                    extra_env,
                    remove_env,
                    templates_dir,
                    helper_available: true,
                };
            }
            return Self::unavailable(templates_dir, extra_env, remove_env);
        }

        Self {
            program: PathBuf::from("pkexec"),
            args: vec![inputs.helper_env.unwrap_or(inputs.host_helper)],
            extra_env,
            remove_env,
            templates_dir,
            helper_available: inputs.host_helper_executable,
        }
    }

    #[must_use]
    pub fn for_tests(fake_helper: PathBuf) -> Self {
        Self {
            program: fake_helper,
            args: Vec::new(),
            extra_env: Vec::new(),
            remove_env: vec!["SHELL".to_string()],
            templates_dir: None,
            helper_available: true,
        }
    }

    #[must_use]
    pub fn unavailable(
        templates_dir: Option<PathBuf>,
        extra_env: Vec<(String, String)>,
        remove_env: Vec<String>,
    ) -> Self {
        Self {
            program: PathBuf::new(),
            args: Vec::new(),
            extra_env,
            remove_env,
            templates_dir,
            helper_available: false,
        }
    }
}

/// True when running inside a Flatpak sandbox.
#[must_use]
pub fn in_flatpak() -> bool {
    std::env::var_os("FLATPAK_ID").is_some() || Path::new("/.flatpak-info").exists()
}

#[must_use]
pub fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(value) => matches!(
            value.to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => false,
    }
}

fn nonempty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn parse_spawn_args(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_else(|_| {
        raw.split_whitespace()
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    })
}

/// Host-only discovery. May use `Path::exists`.
fn discover_helper_path(helper_env: Option<&str>) -> String {
    if let Some(path) = helper_env {
        return path.to_string();
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let relative = dir.join(HELPER_NAME);
            if relative.exists() {
                return relative.to_string_lossy().into_owned();
            }
        }
    }

    for path in SYSTEM_HELPER_PATHS {
        if Path::new(path).exists() {
            return (*path).to_string();
        }
    }

    HELPER_NAME.to_string()
}

/// Probe host helper paths via `flatpak-spawn`. Never `Path::exists`.
fn probe_flatpak_helper(helper_env: Option<&str>) -> Option<String> {
    let mut candidates = Vec::new();
    if let Some(path) = helper_env {
        candidates.push(path.to_string());
    }
    for path in SYSTEM_HELPER_PATHS {
        candidates.push((*path).to_string());
    }

    candidates.into_iter().find(|path| host_executable(path))
}

fn host_executable(path: &str) -> bool {
    Command::new("flatpak-spawn")
        .args(["--host", "--", "test", "-x", path])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn helper_is_executable(path: &str) -> bool {
    let candidate = Path::new(path);
    if candidate.is_absolute() || path.contains('/') {
        return unix_executable(candidate);
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| unix_executable(&dir.join(path))))
        .unwrap_or(false)
}

fn unix_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .ok()
        .is_some_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_inputs() -> SpawnInputs {
        SpawnInputs {
            no_pkexec: false,
            spawn_program: None,
            spawn_args: Vec::new(),
            helper_env: None,
            templates_dir: None,
            in_flatpak: false,
            probed_helper: None,
            host_helper: "/usr/bin/nixos-toolkit-helper".into(),
            host_helper_executable: true,
        }
    }

    #[test]
    fn flatpak_without_helper_does_not_pkexec() {
        let mut inputs = base_inputs();
        inputs.in_flatpak = true;
        inputs.probed_helper = None;
        let spec = SpawnSpec::from_parts(inputs);
        assert!(!spec.helper_available);
        assert_ne!(spec.program.as_os_str(), std::ffi::OsStr::new("pkexec"));
        assert!(!spec.args.iter().any(|a| a == "pkexec"));
    }

    #[test]
    fn flatpak_with_helper_uses_flatpak_spawn_and_pkexec() {
        let mut inputs = base_inputs();
        inputs.in_flatpak = true;
        inputs.probed_helper = Some("/run/current-system/sw/bin/nixos-toolkit-helper".into());
        let spec = SpawnSpec::from_parts(inputs);
        assert!(spec.helper_available);
        assert_eq!(spec.program, PathBuf::from("flatpak-spawn"));
        assert!(spec.args.iter().any(|a| a == "pkexec"));
        assert!(spec.args.iter().any(|a| a == "--forward-fd=0"));
        assert!(spec.args.iter().any(|a| a == "--host"));
    }

    #[test]
    fn no_pkexec_spawns_helper_directly() {
        let mut inputs = base_inputs();
        inputs.no_pkexec = true;
        inputs.host_helper = "/tmp/fake-helper".into();
        let spec = SpawnSpec::from_parts(inputs);
        assert_eq!(spec.program, PathBuf::from("/tmp/fake-helper"));
        assert!(spec.args.is_empty());
        assert!(spec.remove_env.iter().any(|k| k == "SHELL"));
    }

    #[test]
    fn spawn_env_overrides_program() {
        let mut inputs = base_inputs();
        inputs.spawn_program = Some("flatpak-spawn".into());
        inputs.spawn_args = vec!["--host".into(), "--".into()];
        inputs.host_helper = "/usr/bin/nixos-toolkit-helper".into();
        let spec = SpawnSpec::from_parts(inputs);
        assert_eq!(spec.program, PathBuf::from("flatpak-spawn"));
        assert_eq!(
            spec.args,
            vec![
                "--host".to_string(),
                "--".to_string(),
                "/usr/bin/nixos-toolkit-helper".to_string()
            ]
        );
    }

    #[test]
    fn host_install_uses_pkexec() {
        let spec = SpawnSpec::from_parts(base_inputs());
        assert!(spec.helper_available);
        assert_eq!(spec.program, PathBuf::from("pkexec"));
        assert_eq!(spec.args, vec!["/usr/bin/nixos-toolkit-helper".to_string()]);
    }

    #[test]
    fn missing_host_helper_does_not_pkexec() {
        let mut inputs = base_inputs();
        inputs.host_helper_executable = false;
        let spec = SpawnSpec::from_parts(inputs);
        assert!(!spec.helper_available);
        assert_eq!(spec.program, PathBuf::from("pkexec"));
    }
}
