# Decisions (audit-hardening)

Priority when specialists disagree: security and data loss → correctness → feature completeness → Flatpak → accessibility → COSMIC conventions → performance → maintainability → cosmetics.

## D1 — Treat README / Fluent / dialog copy as the contract

Advertised behaviour that looks like a “bug” in a code review stays unless it is actively misleading.

**Keep**

- NVIDIA detection defaults an unwritten driver to stable and makes Apply non-empty (`BUG-001` as filed). Hardware page must still show that default.
- Writing `hardware.nix` also emits default PipeWire (`BUG-002`). Do not silently omit audio.
- Empty Apply is a destructive reset of managed toolkit files; the dialog already warns (`BUG-003`).
- Catalog ids are not added as custom packages (`BUG-009`).
- GUI does not send `Validate` / `Generate` / `CheckPermissions` / `GetSystemInfo` (`BUG-028`). Helper still implements them. Preview stays local.
- IPv4-only DNS (`BUG-042`). Tighten the field description so it says IPv4.

**Change** because the UI/docs promise the opposite

- Unchecking a catalog package must affect **modules**, not only `environment.systemPackages` (`BUG-007`).
- “Open Source (nvidia-open)” must produce `hardware.nvidia.open = true` (`BUG-018`).
- `last_applied` must be written on successful Apply (`BUG-021`).

## D2 — Helper is the TCB; re-validate every Apply/Generate payload

Polkit auth is not a substitute for schema checks. After `pkexec`, stdin is still untrusted relative to Nix evaluation.

- Sanitize or omit user strings in Nix comments (`SEC-001`).
- Shared validators in `common` for hostname, DNS, username, groups, package attrpaths.
- Helper Apply/Generate/WriteState reject invalid payloads.
- Cap JSON line length.
- Do **not** split polkit into three live `--action-id`s in this pass (policy/path matching is fragile across Nix store vs `/run/current-system`). Document the multiplexed helper. Optional later.

## D3 — Stream helper events; do not redesign IPC

`session::stream` must yield `HelperEvent`s as the client reads them, via `spawn_blocking` + a channel. Keep one process per op and the Apply chain. Do not nest `Message`.

Apply remains unbounded (rebuilds can take minutes) but **must** surface a Cancel that kills the child. List/disk/maintenance timeouts must toast and clear busy (raise maintenance/disk timeout; `du` should not run on every Maintenance visit if we can avoid a full store walk — prefer `nix-store --df` or cached size, else longer timeout + live log).

## D4 — Do not rewrite the Application

Keep flat `Message`, `AppModel::apply` → `Intent`, `nav_bar`, settings pages, Fluent. Remove dead Intents (`SendOnSession` / `CloseSession`) if nothing constructs them. Do not nest page enums.

## D5 — COSMIC: fix composition, not the toolkit choice

Use `settings::item` togglers (Services), pin the status banner outside the page scrollable, stop rebuilding `Toasts` on dismiss, prefer `flex_control` on wide value widgets, remove View → Quit (window chrome quits). Bundle expander stays custom (libcosmic has no expander).

## D6 — NVIDIA open driver

Combo index 2 (`nvidia-open`) sets `nvidia_open = true` in state when selected, and `generate_hardware_nix` forces `open = true` for that index. The separate “open kernel modules” toggler remains for stable/beta.

## D7 — WireGuard UDP ports

Enabling WG opens the listen port. Changing the listen port **moves** the auto-opened port (remove previous auto port if it is not a user chip). Disabling WG removes the auto-opened listen port only.

## D8 — Packaging

- Inherit workspace `repository` on the GUI crate so About’s Repository link is not empty.
- Drop Flatpak `--filesystem=/etc/nixos:ro` (detection already uses `flatpak-spawn --host`). Keep `/etc/NIXOS:ro` if still used without spawn; prefer spawn-only and drop bind mounts that duplicate probes when tests allow.
- Update metainfo from “skeleton” to the current product.
- `scripts/verify.sh` gains `cargo fmt --check` plus desktop/appstream validation when tools exist (fail clearly if missing).
- One English Fluent file (`gui.ftl`); delete the duplicate `nixos_toolkit.ftl`.
- `cargo fmt` the two baseline-dirty files.

## D9 — Out of scope / deferred with justification

| Item | Why deferred |
|---|---|
| Full IPv6 DNS | Documented IPv4-only; copy fix only |
| Per-op polkit `--action-id` | Store path vs profile path; easy to break Apply |
| Non-English locales | Explicit limitation |
| WireGuard peers/keys | Explicit limitation (secrets) |
| Flathub upload / real screenshot hosting | Need generated UI captures; metainfo caption can stay until a PNG exists |
| Splitting `apply.rs` | Half tests; freeze contract; not required for correctness |
| `du -sh /nix/store` replacement with a perfect async store walker | Raise timeout, don’t auto-hit on every nav, show errors; full store accounting is the helper’s job |
| Test/Build not writing managed files | `nixos-rebuild` must see the tree; Dry Run already restores. Document Test/Build persist files |

## D10 — Dirty ReadState

If `has_changes` is true when `HelperResponse::State` arrives, keep GUI state and banner a warning instead of silently replacing. Startup with no edits still loads `state.json`.
