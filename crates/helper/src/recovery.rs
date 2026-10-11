//! Durable apply recovery. The journal outlives the helper and protects the
//! previous closures from GC until state is saved or recovery succeeds.
use anyhow::{bail, Context, Result};
use common::ipc::{HelperResponse, LogLevel, RebuildType};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

const PARENT: &str = "/var/lib/nixos-toolkit";
const JOURNAL: &str = "/var/lib/nixos-toolkit/recovery";
const ROOTS: &str = "/nix/var/nix/gcroots/nixos-toolkit-recovery";
const SYSTEM: &str = "/nix/var/nix/profiles/system";

#[derive(Serialize, Deserialize)]
struct Record {
    mode: RebuildType,
    existed: bool,
    running: Option<PathBuf>,
    selected: Option<PathBuf>,
    awaiting_state: bool,
    started: bool,
    files_restored: bool,
}

fn resolved(path: &str) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn read() -> Result<Option<Record>> {
    crate::commands::reject_symlink_tree(Path::new(JOURNAL))?;
    let path = Path::new(JOURNAL).join("record.json");
    match fs::read(&path) {
        Ok(data) => Ok(Some(serde_json::from_slice(&data)?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if Path::new(JOURNAL).exists() {
                bail!("Recovery record is incomplete at {JOURNAL}; retain it for manual recovery")
            }
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

fn write_at(directory: &Path, record: &Record) -> Result<()> {
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    serde_json::to_writer(&mut file, record)?;
    file.flush()?;
    file.as_file().sync_all()?;
    file.persist(directory.join("record.json"))?;
    File::open(directory)?.sync_all()?;
    Ok(())
}

fn private_directory(path: &Path) -> Result<()> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            bail!(
                "Recovery directory is not a regular directory: {}",
                path.display()
            );
        }
    }
    fs::create_dir_all(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

pub fn pending() -> Result<bool> {
    Ok(read()?.is_some())
}

pub fn awaiting_state() -> Result<bool> {
    Ok(read()?.is_some_and(|record| record.awaiting_state))
}

pub fn begin(backup: Option<&Path>, mode: RebuildType) -> Result<()> {
    if pending()? {
        bail!("An earlier apply needs recovery before another operation can start")
    }
    private_directory(Path::new(PARENT))?;
    let staging = tempfile::Builder::new()
        .prefix(".recovery-")
        .tempdir_in(PARENT)?;
    fs::set_permissions(staging.path(), fs::Permissions::from_mode(0o700))?;
    if let Some(backup) = backup {
        crate::commands::copy_dir_all(backup, &staging.path().join("managed"))?;
        sync_tree(staging.path())?;
    }
    let record = Record {
        mode,
        existed: backup.is_some(),
        running: resolved("/run/current-system").filter(|p| store_system(p)),
        selected: resolved(SYSTEM).filter(|p| store_system(p)),
        awaiting_state: false,
        started: false,
        files_restored: false,
    };
    write_at(staging.path(), &record)?;
    fs::rename(staging.path(), JOURNAL)?;
    File::open(PARENT)?.sync_all()?;
    // These are direct Nix GC roots, independent of profile-generation entries.
    // If root creation fails, the durable backup remains and apply is blocked.
    private_directory(Path::new(ROOTS))?;
    for (name, target) in [("running", &record.running), ("selected", &record.selected)] {
        if let Some(target) = target {
            if !store_system(target) {
                // Namespace fixtures can omit store closures. A real NixOS host
                // must have a store-backed system before activation is allowed.
                bail!(
                    "Previous system is not a Nix store closure: {}",
                    target.display()
                )
            }
            let root = Path::new(ROOTS).join(name);
            if root.symlink_metadata().is_ok() {
                fs::remove_file(&root)?;
            }
            symlink(target, root)?;
        }
    }
    File::open(ROOTS)?.sync_all()?;
    Ok(())
}

fn store_system(path: &Path) -> bool {
    path.parent() == Some(Path::new("/nix/store"))
        && path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().contains("-nixos-system-"))
}

pub(crate) fn sync_tree(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            sync_tree(&path)?;
        } else {
            File::open(path)?.sync_all()?;
        }
    }
    File::open(path)?.sync_all()?;
    Ok(())
}

pub fn start_rebuild() -> Result<()> {
    let mut record = read()?.context("Missing apply recovery record")?;
    record.started = true;
    write_at(Path::new(JOURNAL), &record)
}

pub fn failed(files_restored: bool) -> Result<()> {
    let Some(record) = read()? else { return Ok(()) };
    if files_restored
        && (!record.started || matches!(record.mode, RebuildType::Build | RebuildType::DryBuild))
    {
        complete()
    } else {
        mark(false, files_restored)?;
        crate::commands::send_log(LogLevel::Warning,
            "Managed-file recovery does not restore activated services or the boot selection. Use Restore previous system before further changes.".into());
        Ok(())
    }
}

pub fn mark(awaiting_state: bool, files_restored: bool) -> Result<()> {
    if let Some(mut record) = read()? {
        record.awaiting_state = awaiting_state;
        record.files_restored = files_restored;
        write_at(Path::new(JOURNAL), &record)?;
    }
    Ok(())
}

pub fn complete() -> Result<()> {
    // Remove the journal first. An orphaned GC root only retains disk space;
    // the inverse ordering could expose an unfinished recovery to collection.
    if Path::new(JOURNAL).exists() {
        crate::commands::reject_symlink_tree(Path::new(JOURNAL))?;
        let retired = tempfile::Builder::new()
            .prefix(".completed-recovery-")
            .tempdir_in(PARENT)?;
        fs::rename(JOURNAL, retired.path().join("completed"))?;
        File::open(PARENT)?.sync_all()?;
    }
    if Path::new(ROOTS).exists() {
        // Expected children are symlinks; never follow them while deleting.
        for name in ["running", "selected"] {
            let path = Path::new(ROOTS).join(name);
            if path.symlink_metadata().is_ok() {
                fs::remove_file(path)?;
            }
        }
        fs::remove_dir(ROOTS)?;
    }
    Ok(())
}

pub fn status() -> HelperResponse {
    match read() {
        Ok(record) => HelperResponse::RecoveryStatus(record.map(|record| common::ipc::RecoveryInfo {
            message: if record.awaiting_state {
                "The rebuild finished, but saving the selection was interrupted. Restore the previous system before making further changes.".into()
            } else {
                "An apply did not finish. Services or the next boot configuration may have changed, even if managed files were restored.".into()
            },
            backup_path: JOURNAL.into(),
            files_restored: record.files_restored,
            previous_running: record.running.map(|p| p.display().to_string()),
            previous_selected: record.selected.map(|p| p.display().to_string()),
            current_running: resolved("/run/current-system").map(|p| p.display().to_string()),
            current_selected: resolved(SYSTEM).map(|p| p.display().to_string()),
        })),
        Err(error) => failure(error),
    }
}

fn run(command: &mut Command) -> Result<()> {
    let output = crate::operation::run(command, |_, message| {
        crate::commands::send_log(LogLevel::Info, message.into());
    })?;
    if !output.status.success() {
        bail!(
            "Recovery command failed with {}; backup retained at {JOURNAL}",
            output.status
        )
    }
    Ok(())
}

fn restore() -> Result<()> {
    let Some(record) = read()? else { return Ok(()) };
    crate::commands::restore_recovery_files(Path::new(JOURNAL), record.existed)?;
    mark(record.awaiting_state, true)?;
    let activates =
        record.started && matches!(record.mode, RebuildType::Switch | RebuildType::Test);
    let boots = record.started && matches!(record.mode, RebuildType::Switch | RebuildType::Boot);
    if activates || boots {
        for path in [&record.running, &record.selected].into_iter().flatten() {
            if !store_system(path) {
                bail!("Invalid recovery system path")
            }
        }
    }
    if activates {
        let running = record.running.as_ref().context(
            "Previous running system could not be identified; manual recovery is required",
        )?;
        run(Command::new(running.join("bin/switch-to-configuration")).arg("test"))?;
        if resolved("/run/current-system").as_ref() != Some(running) {
            bail!("Running system did not return to its previous closure")
        }
    }
    if boots {
        let selected = record.selected.as_ref().context(
            "Previous boot selection could not be identified; manual recovery is required",
        )?;
        run(Command::new("nix-env")
            .args(["--profile", SYSTEM, "--set"])
            .arg(selected))?;
        run(Command::new(selected.join("bin/switch-to-configuration")).arg("boot"))?;
        if resolved(SYSTEM).as_ref() != Some(selected) {
            bail!("Boot selection did not return to its previous closure")
        }
    }
    complete()?;
    Ok(())
}

pub fn recover() -> HelperResponse {
    match restore() {
        Ok(()) => HelperResponse::Ok,
        Err(error) => failure(error),
    }
}

fn failure(error: anyhow::Error) -> HelperResponse {
    HelperResponse::Error {
        message: "System recovery needs attention".into(),
        details: Some(format!("{error}; backup: {JOURNAL}")),
    }
}
