# Security and robustness review (reconnaissance)

**Scope:** NixOS Toolkit (`nixos-toolkit` GUI + `nixos-toolkit-helper`), branch `audit-hardening`.  
**Date:** 2026-09-12.  
**Mode:** Reconnaissance only. No production code was changed.

This review treats the **helper as the TCB**. The GUI is unprivileged and must not be trusted to sanitize inputs that become Nix, paths, or subprocess arguments. Polkit authentication is the privilege boundary; after `pkexec` the helper currently performs any `HelperRequest`.

## Method

- Trust-boundary walk: files/paths, Nix generation, IPC JSON-lines, process spawn, env vars, temp files, polkit, Flatpak finish-args, portals/clipboard/URLs.
- Primary code: `crates/helper/src/{commands,nix_gen,rebuild,main}.rs`, `crates/common/src/{ipc,nix,config}.rs`, `crates/gui/src/helper/{spawn,client,session}.rs`, `crates/gui/src/{integration,core/apply,core/packages,app,config}.rs`.
- Policy/packaging: `data/polkit/org.nixos-toolkit.helper.policy`, `flake.nix` (polkit `substituteInPlace` + `wrapProgram`), `flatpak/io.github.goshitsarch_eng.NixosToolkit.yml`.
- Pattern search: `Command::new`, `pkexec`, `flatpak-spawn`, `unsafe`, path joins, `unwrap`/`expect`.
- **`cargo audit` was not executed in this pass** (no interactive `nix build` / `cargo audit` in the recon environment). The flake defines `checks.<system>.audit` via `craneLib.cargoAudit`. Run that (or `cargo audit`) before treating dependency risk as clear.

## Architecture (security-relevant)

```
unprivileged nixos-toolkit
  JSON lines stdin/stdout
  pkexec | flatpak-spawn --host … pkexec | NIXOS_TOOLKIT_NO_PKEXEC
privileged nixos-toolkit-helper
  writes /etc/nixos/nixos-toolkit/**
  nixos-rebuild, nix-env (system profile), allowlisted maintenance
```

GUI writes only `$XDG_CONFIG_HOME/nixos-toolkit/preferences.json` (theme). It does not write `/etc/nixos` (confirmed: `fs::write` under `crates/gui/src` is prefs/tests only). Local preview uses `common::nix::generate_preview_full_from` and does not call the helper.

## What already works

| Control | Evidence |
|---|---|
| Write jail | `paths::is_allowed_managed_path` + `atomic_write` refuse non-absolute paths, `..`, and targets whose existing prefix canonicalizes outside `/etc/nixos/nixos-toolkit`. Tests in `config.rs` / `nix_gen.rs`. |
| No shell for rebuild/maintenance | `Command::new` + argv; maintenance is exact-string allowlist (6 commands) then `split_whitespace`. |
| Nix string quoting | `nix_escape_string` on hostname, DNS values, username, groups. `$` escaped. `is_nix_attrpath` drops `;` / spaces / `..` in package attrs. `PermitRootLogin` allowlisted. |
| Profile/bundle write paths | Dest `profiles/{id}.nix` / `bundles/{id}.nix` only for **catalog** ids after lookup, not raw IPC strings. |
| Rollback `activate` | `"switch"` / `"boot"` only. |
| GUI does not spawn pkexec when helper missing | `helper_available`; Flatpak probe uses `flatpak-spawn --host -- test -x`. |
| `SHELL` stripped | Avoids pkexec rejecting nix-develop shells. |
| Desktop files | No MIME/URI handlers. No drag-and-drop. Clipboard is snippets/preview only. |
| Flatpak (positive) | No `--share=network`, no `--filesystem=host`, no `--device=all`. |
| Secrets | No passwords/keys in `AppState`. SSH options are booleans/enums. |
| `unsafe` | None in first-party Rust. |

---

## Findings

### SEC-001 — Nix comment breakout in `selected.nix` (helper TCB)

| | |
|---|---|
| **Severity** | **High** |
| **Class** | C (external/IPC input must not become Nix syntax) |

**Evidence.** `generate_selected_nix_full` interpolates user-controlled lists into `#` comments without escaping newlines:

```197:250:crates/common/src/nix.rs
        r#"# NixOS Toolkit - Managed Configuration
...
# DNS servers: {}
# User groups: {}
...
# Custom packages: {}
```

`dns_servers`, `user_groups`, and `custom_packages` are joined raw. A payload such as `1.1.1.1\n};\n  users.users.root.hashedPassword = "...";\n#` terminates the comment and injects Nix into `selected.nix`, which the user is instructed to `imports`.

Hostname/DNS **values** in snippet files are quoted via `nix_escape_string`. The comment header is not. `generate_fallback_bundle` also interpolates `id` into a comment (catalog-only in the helper write path).

GUI `parse_and_set_dns` / package parser block most of this **on the GUI path only**. `Apply` / `Generate` in the helper never call `validate()` and do not re-run those parsers.

**Expected.** Helper rejects or sanitizes every string written into Nix (comments included): no CR/LF in comments, or omit user data from comments.

**Fix.** (1) Stop interpolating untrusted strings into comments, or replace `\n`/`\r` and `#`. (2) On `Apply`/`Generate`, enforce the same rules as the GUI: ASCII hostname, IPv4/IPv6 DNS, username charset, `is_nix_attrpath` on packages/groups. (3) Tests with newline/`${`/`"` in each field.

---

### SEC-002 — Helper is a multiplexed root agent that trusts stdin

| | |
|---|---|
| **Severity** | **High** (architectural) |
| **Class** | Design / confused deputy |

**Evidence.** After `pkexec`, `handle_request` dispatches every `HelperRequest` with no caller identity check, no per-op polkit action, and no size cap:

- `Apply` / `Generate { dry_run: false }` write managed Nix and (Apply) run `nixos-rebuild`.
- `WriteState` writes arbitrary `AppState` JSON.
- `RollbackGeneration` / `DeleteGenerations` mutate the system profile.
- `RunMaintenance` runs GC / store verify / flake update (allowlisted).
- `GetDiskUsage` runs `du -sh /nix/store`.

One helper process per GUI op; Apply chain is `EnsureDirectories` → `Apply` → `WriteState` on the **same** stdin. Any code that can write that pipe after auth (compromised GUI process, injected `NIXOS_TOOLKIT_SPAWN` wrapper in the user session) gets full toolkit privilege.

`Validate` exists (`commands.rs`) but is unused by Apply/Generate and unused by the GUI.

**Expected.** Destructive ops re-validate payloads. Prefer distinct polkit actions (or a capability argument) for write vs rebuild vs generation delete vs GC. Optional: bind username/groups to `PKEXEC_UID`.

**Fix.** Shared `validate_apply_payload()` used by Apply/Generate/WriteState. Reject `Generate` with `dry_run: false` or treat it as Apply-without-rebuild only after the same checks. Cap JSON line length. Do not log full request bodies.

---

### SEC-003 — Polkit policy does not match how the GUI invokes pkexec

| | |
|---|---|
| **Severity** | **Medium** |
| **Class** | Privilege duration / policy dead code |

**Evidence.** Three actions (`manage-system`, `write-config`, `rebuild`) all annotate the same `org.freedesktop.policykit.exec.path`. GUI spawn is `pkexec <helper-path>` with **no** `--action-id`.

- `manage-system` and `write-config`: `allow_active` = `auth_admin_keep`.
- `rebuild`: `allow_active` = `auth_admin` (no keep).

pkexec matches by executable path, not by intended operation. The rebuild-without-keep action is effectively unused. Credential keep on the first matching action means a later `nixos-rebuild switch`, generation delete, or `nix-collect-garbage -d` can proceed without a new prompt for the keep window.

`allow_gui` is true (needed for a desktop helper). `allow_any` / `allow_inactive` are `auth_admin` (SSH/inactive sessions can still auth as admin — typical, but broad).

Flake `substituteInPlace` rewrites the canned `/run/current-system/sw/bin/…` path to `$out/bin/nixos-toolkit-helper` (wrapped). That is the correct store path for a Nix install; a raw copy of `data/polkit/…` without substitute would not match the wrapped binary.

**Expected.** One action per privilege class, invoked with `--action-id`, `exec.argv` constraints if polkit version allows, and **no keep** (or short keep) for rebuild / delete-generations / GC `-d`.

**Fix.** Collapse to one documented action **or** split and pass `--action-id`. Set destructive actions to `auth_admin` without keep. Document that any local admin who can pkexec the helper can perform every helper op.

---

### SEC-004 — `atomic_write` / dry-run copy: symlink and temp-name issues

| | |
|---|---|
| **Severity** | **Medium** (High if the managed directory is ever writable by non-root) |
| **Class** | B/C — filesystem races |

**Evidence.**

1. Temp name is `path.with_extension("tmp")` (`hostname.nix` → `hostname.tmp`, `state.json` → `state.tmp`), not a unique `O_EXCL` name. Check `is_allowed_managed_path` then `fs::write` then `rename`. If `hostname.tmp` is replaced with a symlink after the check, `fs::write` follows it (classic write-jail TOCTOU).
2. `fs::write` / `rename` do not use `O_NOFOLLOW`.
3. `copy_dir_all` (dry-build snapshot) uses `file_type()?.is_dir()` then `fs::copy`, which **follows file symlinks** and can pull `/etc/shadow` (or huge files) into the snapshot if a symlink exists in the managed tree.
4. `create_dir_all` / `fs::write` do not set explicit modes (umask-dependent). World-writable managed dirs would make (1)–(3) exploitable by a local user.

Helper typically runs as root with umask `022` → `755`/`644`. Unprivileged users should not win the race **if** ownership is correct.

**Expected.** `openat` + `O_NOFOLLOW` + unique temp in the same directory, `fsync`, rename. Refuse to snapshot/copy/delete through symlinks. `DirBuilder` `0o755`, files `0o644`.

**Fix.** `tempfile` in the destination dir (already a dependency) or `libc` `O_NOFOLLOW`. Skip/error on `symlink_metadata` in `copy_dir_all` and `delete_unlisted_nix`. Explicit chmod. Tests with dangling and escaping symlinks.

---

### SEC-005 — Unbounded helper stdin / IPC payloads (DoS as root)

| | |
|---|---|
| **Severity** | **Medium** |
| **Class** | C — invalid/oversized external input |

**Evidence.** `main.rs` reads `stdin.lock().lines()` with no max length. `Apply` can carry unbounded `Vec`s (`custom_packages`, ports, `bundle_packages`). `WriteState` can serialize a huge `AppState`. A client that already passed pkexec (or `NO_PKEXEC` as root) can OOM the helper.

GUI `HelperClient` also parses helper stdout lines unbounded and logs the raw line on JSON failure (`client.rs`).

`GetDiskUsage` runs `du -sh /nix/store` as root (I/O heavy). Maintenance `nix-store --verify --check-contents` similarly. No concurrency lock if the user launches two GUIs (single-instance is off, DECISIONS C22).

**Expected.** Reject lines over a few MiB. Cap list lengths. Serialize `du`/verify behind a timeout. Optional flock on `/run/nixos-toolkit-helper.lock`.

**Fix.** `take` / byte cap before `from_str`. Limits on `custom_packages`, ports, generations vec. Timeout on `du`.

---

### SEC-006 — Privileged `Command::new("relative-name")` (PATH)

| | |
|---|---|
| **Severity** | **Medium** (unpackaged / `NO_PKEXEC`); **Low** for Nix-wrapped helper |
| **Class** | B |

**Evidence.** Helper executes `nixos-rebuild`, `nix-env`, `du`, and allowlisted `nix-collect-garbage` / `nix-store` / `nix` / `nix-channel` by **name**. `which()` for `can_run_rebuild` only checks `is_file()`, not the executable bit.

`flake.nix` `wrapProgram` prefixes `PATH` with `nix`, `nixos-rebuild`, `git`, `hostname`. pkexec also uses a secure path, then the wrapper re-extends it. Dev `cargo run -p helper` as root with a user `PATH` would be hijackable.

`rebuild.rs` flake ref is `format!("/etc/nixos#{}", hostname)` from `/etc/hostname` as a single `Command` arg (no shell). Unusual hostname characters cannot inject extra argv; they can only select a weird flake attribute.

**Expected.** Absolute paths (`/run/current-system/sw/bin/nixos-rebuild` or wrapped store paths). `which()` should require `+x`.

---

### SEC-007 — Flatpak `--talk-name=org.freedesktop.Flatpak` (host spawn)

| | |
|---|---|
| **Severity** | **Medium** (accepted for helper; residual read/exec on host) |
| **Class** | Sandbox |

**Evidence.** `finish-args` include `org.freedesktop.Flatpak` (with linter skip) plus `--filesystem=/etc/nixos:ro` (and `/etc/NIXOS`, `/etc/hostname`). GUI uses `flatpak-spawn --host` for:

- `pkexec` helper (`spawn.rs`)
- `test -e` / `cat` of **fixed** paths (`integration.rs`)
- `hostname`, `lspci`
- `xdg-open /etc/nixos` (`app.rs` `open_path`)

Host `cat` is not user-path-controlled today. The permission still lets a **compromised GUI** read any host file the user can read and run any host command (including `pkexec`). That is stronger than the bind-mount of `/etc/nixos`.

About URLs use `open::that_detached` **inside** the sandbox (`Intent::OpenUrl`), not host `xdg-open`. No scheme allowlist (`https` only).

**Expected.** Keep host spawn only for pkexec + a tiny allowlist of host paths. Prefer portals for opening `/etc/nixos`. Allowlist `https://` for `OpenUrl`.

**Fix.** Document the sandbox hole in user-facing install notes. Do not add `--filesystem=home` or `--share=network` without a new review. Scheme-check URLs.

---

### SEC-008 — `Generate { dry_run: false }` writes managed files without rebuild

| | |
|---|---|
| **Severity** | **Medium** |
| **Class** | Extra privileged capability |

**Evidence.** `commands::generate` calls `ensure_directories` and `generate_all_files(..., dry_run)` which writes when `dry_run` is false. GUI preview is local; `to_generate_request` is only used in tests. Any IPC client can still persist Nix under `/etc/nixos/nixos-toolkit` without `nixos-rebuild`.

**Fix.** Ignore `dry_run: false` (force true) or require the same validation as Apply and log a warning. Prefer deleting the write path from `Generate`.

---

### SEC-009 — Dangerous-but-intentional Nix modules (footguns)

| | |
|---|---|
| **Severity** | **Medium** (product policy, not memory unsafety) |
| **Class** | User-triggered system hardening regressions |

Generated/copied modules can:

- Set `services.openssh.settings.PasswordAuthentication` and `PermitRootLogin` to `"yes"` (GUI allowlist includes `"yes"`).
- Set global `nixpkgs.config.allowUnfree = true` (`generate_unfree_nix`, VirtualBox stub/template).
- Open Steam Remote Play / dedicated-server firewall ports (`bundle_module_stub` `"gaming"` and `nix/templates/bundles/gaming.nix`).
- Enable `services.syncthing`, `virtualisation.docker`, `libvirtd` `runAsRoot`, `system.autoUpgrade.enable` with **no** flake/channel URL, dates, or user isolation.
- Enable NVIDIA proprietary drivers and `security.rtkit` via hardware/audio snippets.

These apply only after admin pkexec + confirm dialog (Apply). They are still easy to enable without explaining network/auth impact.

**Fix.** Confirm-dialog callouts for SSH password/root, auto-upgrade, unfree, and firewall-opening bundles. Consider `allowUnfreePredicate` instead of global `allowUnfree`. Do not default Steam firewall opens when the user only wanted a launcher.

---

### SEC-010 — Helper `validate()` is weaker than GUI and unused on Apply

| | |
|---|---|
| **Severity** | **Low** |
| **Class** | C |

`validate()` uses `c.is_alphanumeric()` (Unicode) rather than GUI `is_ascii_alphanumeric`. Hostname may start/end with `-`. Empty vs missing hostname treated differently. Bundles unknown → **warning** not error. Apply never calls it.

**Fix.** Single shared validator in `common`; Apply/Generate return `Error` on failure. ASCII + RFC 1123 (no leading/trailing hyphen).

---

### SEC-011 — Template path join is not jailed

| | |
|---|---|
| **Severity** | **Low** |
| **Class** | C if `NIXOS_TOOLKIT_TEMPLATES_DIR` or catalog is attacker-controlled |

`read_template_from` is `templates_dir.join(template_path)` with no `..` check. Catalog paths are compiled-in (`profiles/gnome.nix`, …). Nix `wrapProgram` sets `NIXOS_TOOLKIT_TEMPLATES_DIR` on **both** GUI and helper to store paths. pkexec typically does **not** forward GUI `extra_env` into the helper; the helper wrapper’s own value wins. Residual risk: unpackaged helper + attacker env.

**Fix.** Canonicalize and require the result to stay under `templates_dir`. Ignore `NIXOS_TOOLKIT_TEMPLATES_DIR` when euid is 0 unless it is a known store/prefix.

---

### SEC-012 — Directory/file modes not explicit

| | |
|---|---|
| **Severity** | **Low** |
| **Class** | B |

`ensure_directories` / `atomic_write` inherit umask. `state.json` is world-readable (no secrets today). Dry-run `TempDir` defaults are restrictive (typically `0700`) — good.

**Fix.** Explicit `0o755` dirs, `0o644` files under the managed tree.

---

### SEC-013 — `OpenUrl` has no scheme allowlist

| | |
|---|---|
| **Severity** | **Low** |
| **Class** | C |

`Message::LaunchUrl` → `open::that_detached` with no `https://` check. Current callers are About drawer links (repo/issues, hardcoded https). If libcosmic ever passed `file://` or `javascript:`, the GUI would launch it.

**Fix.** Allow only `http`/`https`. Keep `OpenPath` as a fixed `/etc/nixos` (already).

---

### SEC-014 — Helper links `tokio` but does not use it

| | |
|---|---|
| **Severity** | **Low** |
| **Class** | Dependency surface |

`crates/helper/Cargo.toml` depends on `tokio` with runtime features; `crates/helper/src` has no `tokio` usage (`std::process::Command` only). Extra crates in a **setuid-equivalent** binary increase advisory exposure.

**Fix.** Drop tokio from the helper crate (keep it on the GUI if libcosmic needs it).

---

### SEC-015 — Development env overrides (`NIXOS_TOOLKIT_*`)

| | |
|---|---|
| **Severity** | **Low** in production wrap; **High** if a user is taught to set `NO_PKEXEC` |
| **Class** | Operator footgun |

`NIXOS_TOOLKIT_NO_PKEXEC`, `NIXOS_TOOLKIT_HELPER`, `NIXOS_TOOLKIT_SPAWN` / `_ARGS`, `NIXOS_TOOLKIT_TEMPLATES_DIR` fully control spawn. Intended for tests (`fake-helper`) and `nix develop`. Production GUI wrap sets `NIXOS_TOOLKIT_HELPER` to the store helper; polkit `exec.path` must match that binary (after flake substitute).

`NIXOS_TOOLKIT_SPAWN` can wrap pkexec (logging stdin) **as the same user** — session malware, not a sandbox escape.

**Fix.** Document that `NO_PKEXEC` is unsupported on production installs. Optionally ignore these vars unless `NIXOS_TOOLKIT_ALLOW_UNSAFE_SPAWN=1` in debug builds.

---

### SEC-016 — GUI apply does not re-check `field_errors`

| | |
|---|---|
| **Severity** | **Low** |
| **Class** | Robustness |

Invalid hostname/DNS/username is kept out of `AppState` (handlers return early). `ConfirmApply` / `RequestDryRun` do not look at `field_errors`; a previous valid value can still be applied while the field shows an error. Not an injection if the stored value was already validated.

**Fix.** Disable Apply while any `field_errors` is `Some`, or re-validate on confirm.

---

### SEC-017 — Naive reconstruct parsers (integrity, not RCE)

| | |
|---|---|
| **Severity** | **Low** (requires write to managed Nix already) |
| **Class** | B |

`parse_selected_nix` / snippet parsers are line-oriented and can mis-parse or pick up `../` fragments. Reconstructed `custom_packages` are filtered again by `is_nix_attrpath` on the next generate. Do not treat reconstruct as authentication of config.

---

### SEC-018 — `cargo audit` not verified this pass

| | |
|---|---|
| **Severity** | **Unknown** until run |
| **Class** | Dependencies |

Flake check: `craneLib.cargoAudit { inherit src advisory-db; }`. GUI pulls **libcosmic git** (large tree: wgpu, wayland, xdg-portal). Helper’s meaningful crates: `serde`, `serde_json`, `anyhow`, `thiserror`, `tracing*`, `tempfile`, unused `tokio`.

**Fix.** `nix build .#checks.x86_64-linux.audit` and `cargo audit` on CI (already in flake checks; GitHub `verify.yml` flake job builds `fmt` and `audit`).

---

## Positive / informational

| ID | Note |
|---|---|
| INFO-001 | GUI never writes `/etc/nixos`. Integration snippets are clipboard text for the user to paste. |
| INFO-002 | Maintenance allowlist is exact strings; flake hosts rewrite `nix-channel --update` → `nix flake update --flake /etc/nixos` only. |
| INFO-003 | `nix_escape_string` + `is_nix_attrpath` + SSH root-login allowlist are the right pattern; extend them to comments and Apply. |
| INFO-004 | No `unsafe`, no URI handlers, no DnD, no network permission in Flatpak. |
| INFO-005 | Dry-build snapshots then restores the managed tree; restore failure becomes `Error` (does not silently leave a new tree). |
| INFO-006 | `HelperRequest` / `HelperResponse` internally tagged; unknown types fail parse (no op smuggling via extra fields as a different variant). |
| INFO-007 | Generation rollback/delete have GUI confirm dialogs; GC `-d` has a warning dialog. Helper does not require those dialogs (SEC-002). |

---

## `unwrap` / `expect` / `panic` classification

Policy used: **A** internal invariant (panic OK), **B** recoverable runtime (must not panic), **C** external/user input (must not panic).

| Site | Class | Notes |
|---|---|---|
| `helper/main.rs` `EnvFilter` `"helper=info".parse().unwrap()` | **A** | Literal directive. |
| `ManagedDirSnapshot::restore` `expect("existed snapshot always has a backup dir")` | **A** | Internal pairing of `existed` + `backup`. |
| `gui/main.rs` same EnvFilter `expect` | **A** | |
| `gui/i18n.rs` fallback language `expect` | **A** | Missing assets = programming error. |
| `core/packages.rs` `chars().next().unwrap()` after empty check | **A** | |
| `nix.rs` test `panic!("missing bundle {id}")` | **A** | tests only |
| Helper/GUI `Command` spawn / `fs` / `serde_json::from_str` on stdin | **B/C** | Already return `HelperResponse::Error` / skip line — **good**. |
| No production `unwrap` on hostname, DNS, JSON IPC, or `nixos-rebuild` output | — | Rebuild uses `map_while(Result::ok)` (drops invalid UTF-8 lines; **B** OK). |

No **C**-class panics found on helper IPC parse (malformed JSON → `Error`). Do **not** add `unwrap` on `Apply` fields.

---

## Suggested fix order

1. **SEC-001 + SEC-002:** helper-side validation + stop putting raw user strings in Nix comments. This is the only plausible “untrusted GUI → root Nix injection” path that does not already require writing `/etc/nixos`.
2. **SEC-004:** `O_NOFOLLOW` / unique tempfile / no symlink follow in snapshot copy; explicit modes.
3. **SEC-003:** polkit `--action-id` and drop `auth_admin_keep` on rebuild-class ops (or document keep as accepted).
4. **SEC-005 / SEC-006 / SEC-008:** line caps, absolute binaries, disable writing `Generate`.
5. **SEC-007 / SEC-009:** document Flatpak host-spawn and add Apply warnings for SSH/unfree/firewall.
6. **SEC-014 / SEC-018:** drop unused tokio; run `cargo audit`.

## Out of scope / not found

- Remote network attack surface on the GUI (no `--share=network`; no listening sockets in first-party code).
- Credential/secret leakage in logs (no secrets in state; do not start logging `Apply` JSON).
- Helper editing `configuration.nix` / `flake.nix` (it does not).
- Desktop/URI handler hijack (none registered).
- First-party `unsafe` / exploit PoCs (none written for this review).
