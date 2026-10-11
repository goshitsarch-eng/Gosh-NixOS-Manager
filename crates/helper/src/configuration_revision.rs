//! Detect stale desktop drafts without disclosing host configuration contents.
use anyhow::{bail, Context, Result};
use common::ipc::HelperResponse;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const MAX_FILES: usize = 4096;
const MAX_BYTES: usize = 32 * 1024 * 1024;

fn field(hash: &mut Sha256, value: &[u8]) {
    hash.update((value.len() as u64).to_le_bytes());
    hash.update(value);
}

fn tree_revision(root: &Path, systems: &[&Path]) -> Result<String> {
    fn visit(
        root: &Path,
        path: &Path,
        depth: usize,
        hash: &mut Sha256,
        count: &mut usize,
        bytes: &mut usize,
    ) -> Result<()> {
        if depth > 32 {
            bail!("Configuration directory nesting exceeds the safety limit");
        }
        let mut entries = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let relative = path.strip_prefix(root)?;
            let kind = entry.file_type()?;
            // Git bookkeeping and editor files do not change the configuration.
            if entry.file_name().as_bytes().starts_with(b".") {
                continue;
            }
            if kind.is_dir() {
                visit(root, &path, depth + 1, hash, count, bytes)?;
                continue;
            }
            // Record symlink targets, but do not recurse outside the config tree.
            if kind.is_symlink() {
                field(hash, b"link");
                field(hash, relative.as_os_str().as_bytes());
                field(hash, fs::read_link(&path)?.as_os_str().as_bytes());
            }
            let tracked = path.extension().is_some_and(|ext| ext == "nix")
                || entry.file_name() == "flake.lock"
                || relative == Path::new("nixos-toolkit/state/state.json");
            if !tracked {
                continue;
            }
            if !fs::metadata(&path)?.is_file() {
                bail!(
                    "Configuration path is not a regular file: {}",
                    path.display()
                );
            }
            *count += 1;
            if *count > MAX_FILES {
                bail!("Too many configuration files to check safely");
            }
            let mut content = Vec::new();
            fs::File::open(&path)
                .with_context(|| format!("Cannot read {}", path.display()))?
                .take((MAX_BYTES - *bytes + 1) as u64)
                .read_to_end(&mut content)?;
            *bytes += content.len();
            if *bytes > MAX_BYTES {
                bail!("Configuration files exceed the 32 MiB safety limit");
            }
            field(hash, b"file");
            field(hash, relative.as_os_str().as_bytes());
            field(hash, &content);
        }
        Ok(())
    }
    let mut hash = Sha256::new();
    visit(root, root, 0, &mut hash, &mut 0, &mut 0)?;
    for system in systems {
        field(&mut hash, system.as_os_str().as_bytes());
        match fs::canonicalize(system) {
            Ok(path) => field(&mut hash, path.as_os_str().as_bytes()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                field(&mut hash, b"missing");
            }
            Err(error) => return Err(error.into()),
        }
    }
    let digest: String = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok(format!("v1:{digest}"))
}

fn current() -> Result<String> {
    tree_revision(
        Path::new("/etc/nixos"),
        &[
            Path::new("/run/current-system"),
            Path::new("/nix/var/nix/profiles/system"),
        ],
    )
}

pub fn check(expected: &str) -> Result<()> {
    if current()? != expected {
        bail!("Configuration files or the running/boot system changed since this draft was loaded. Reload saved selections and review the changes before trying again. Move any edits to Toolkit-generated files into a separate NixOS module first.");
    }
    Ok(())
}

pub fn read_state_snapshot() -> HelperResponse {
    let read = || -> Result<HelperResponse> {
        let revision = current()?;
        let state = match crate::commands::read_state() {
            HelperResponse::State(state) => state,
            error => return Ok(error),
        };
        if current()? != revision {
            bail!("Configuration changed while loading; reload saved selections and try again");
        }
        Ok(HelperResponse::StateSnapshot { state, revision })
    };
    read().unwrap_or_else(|error| HelperResponse::Error {
        message: "Could not safely load the host configuration".into(),
        details: Some(error.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_edits_deletions_state_and_boot_changes_but_ignores_git() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("config");
        fs::create_dir_all(root.join("nixos-toolkit/state")).unwrap();
        fs::write(root.join("configuration.nix"), "original").unwrap();
        let boot = temp.path().join("boot");
        let revision = || tree_revision(&root, &[&boot]).unwrap();
        let original = revision();
        fs::create_dir(root.join(".git")).unwrap();
        fs::write(root.join(".git/index"), "bookkeeping").unwrap();
        assert_eq!(revision(), original);
        fs::write(root.join("configuration.nix"), "edited").unwrap();
        assert_ne!(revision(), original);
        fs::remove_file(root.join("configuration.nix")).unwrap();
        assert_ne!(revision(), original);
        fs::write(root.join("configuration.nix"), "original").unwrap();
        assert_eq!(revision(), original);
        let state = root.join("nixos-toolkit/state/state.json");
        fs::write(&state, "{}").unwrap();
        assert_ne!(revision(), original);
        fs::remove_file(state).unwrap();
        std::os::unix::fs::symlink(&root, &boot).unwrap();
        assert_ne!(revision(), original);
    }
}
