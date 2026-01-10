# NixOS Toolkit - Bug Fixes Progress

## Issue
Changes made in the app were not being saved to NixOS. Two bugs identified:
1. GUI state not persisted between sessions
2. System configuration not being applied properly

## Root Cause Analysis

### Bug 1: State Not Persisted
- The infrastructure for state persistence existed (`ReadState`/`WriteState` IPC messages, `read_state()`/`write_state()` helper functions)
- **Problem**: The GUI never called these - no load on startup, no save after apply

### Bug 2: Config Not Applied
- Files were being written to `/etc/nixos/nixos-toolkit/`
- **Problems**:
  - No verification that files were written correctly
  - No logging of which files were generated
  - Integration detection was too weak (checked for "nixos-toolkit" string instead of exact import path)

### Bug 3: Helper Binary Name Mismatch
- The GUI code expected binary named `nixos-toolkit-helper`
- Cargo.toml had `name = "helper"` which produced binary named `helper`
- **Fixed**: Changed Cargo.toml to `name = "nixos-toolkit-helper"`

## Fixes Implemented

### 1. State Loading on Startup
**File**: `crates/gui/src/window.rs`

Added `load_state()` method that:
- Spawns the privileged helper on app startup
- Sends `HelperRequest::ReadState`
- Populates `app_state` with loaded selections
- Handles failures gracefully (logs warning, continues with defaults)

```rust
fn load_state(&self) {
    match HelperClient::spawn_privileged() {
        Ok(mut client) => {
            client.send(&HelperRequest::ReadState)?;
            match client.recv_timeout(Duration::from_secs(5)) {
                Some(HelperResponse::State(ipc_state)) => {
                    *self.imp().app_state.borrow_mut() = AppState::from_ipc_state(ipc_state);
                }
                // ... error handling
            }
        }
        // ... error handling
    }
}
```

### 2. State Saving After Apply
**File**: `crates/gui/src/pages/apply.rs`

Added `save_state()` method called from `finish_apply()` when successful:
- Gets current state from window
- Converts to IPC format
- Sends `HelperRequest::WriteState` to helper
- Logs success/failure

### 3. File Write Logging
**File**: `crates/helper/src/commands.rs`

Modified `apply()` function to log each file written:
```rust
Ok(files) => {
    for f in &files {
        send_log(LogLevel::Info, format!("Wrote: {}", f.path));
    }
    send_log(LogLevel::Info, format!("Generated {} configuration files", files.len()));
}
```

### 4. File Write Verification
**File**: `crates/helper/src/nix_gen.rs`

Added verification in `atomic_write()`:
```rust
// Verify the write succeeded by reading back
let written = fs::read_to_string(path)?;
if written != content {
    anyhow::bail!("Write verification failed for {}: content mismatch", path.display());
}
```

### 5. Improved Integration Detection
**File**: `crates/helper/src/commands.rs`

Changed from weak string matching to exact path checking:
```rust
if content.contains("./nixos-toolkit/state/selected.nix")
    || content.contains("/etc/nixos/nixos-toolkit/state/selected.nix")
    || content.contains("nixos-toolkit/state/selected.nix")
{
    return IntegrationStatus::Integrated;
}
```

### 6. Fixed Helper Binary Name
**File**: `crates/helper/Cargo.toml`

Changed binary name to match what GUI expects:
```toml
[[bin]]
name = "nixos-toolkit-helper"  # Was "helper"
path = "src/main.rs"
```

## Files Modified

| File | Changes |
|------|---------|
| `crates/gui/src/window.rs` | Added imports, `load_state()` method, call from `constructed()` |
| `crates/gui/src/pages/apply.rs` | Added `save_state()` method, call from `finish_apply()` |
| `crates/helper/src/commands.rs` | Added `send_log()`, file logging in `apply()`, improved `detect_integration()` |
| `crates/helper/src/nix_gen.rs` | Added write verification in `atomic_write()` |
| `crates/helper/Cargo.toml` | Changed binary name from `helper` to `nixos-toolkit-helper` |

## Build Verification

Code compiles successfully:
```bash
nix --extra-experimental-features 'nix-command flakes' develop --command cargo check
```

## How to Run (Testing)

### Option 1: Development Mode (with helper) - RECOMMENDED
```bash
cd /home/gosh/Downloads/NixOSApp-main

# Enter nix develop shell and build helper
nix --extra-experimental-features 'nix-command flakes' develop --command cargo build -p helper

# Run with helper path set
nix --extra-experimental-features 'nix-command flakes' develop --command bash -c \
  'export NIXOS_TOOLKIT_HELPER="$PWD/target/debug/nixos-toolkit-helper" && cargo run -p gui'
```

### Option 2: Run as Root (if no polkit agent)
If pkexec doesn't show password prompt, run as root:
```bash
cd /home/gosh/Downloads/NixOSApp-main
sudo nix --extra-experimental-features 'nix-command flakes' develop --command bash -c \
  'export NIXOS_TOOLKIT_HELPER="$PWD/target/debug/nixos-toolkit-helper" && cargo run -p gui'
```

### Option 3: Full Build
```bash
# Build everything
nix --extra-experimental-features 'nix-command flakes' build

# Run the installed binary
./result/bin/gui
```

## Polkit Requirement

The app uses `pkexec` for privilege escalation. This requires a **polkit authentication agent** to be running to display password prompts.

If clicking "Apply" fails with "Unexpected response from helper", you likely don't have a polkit agent running.

### Fix: Add polkit-gnome to NixOS configuration
```nix
# In configuration.nix
security.polkit.enable = true;

systemd.user.services.polkit-gnome-authentication-agent-1 = {
  description = "polkit-gnome-authentication-agent-1";
  wantedBy = [ "graphical-session.target" ];
  wants = [ "graphical-session.target" ];
  after = [ "graphical-session.target" ];
  serviceConfig = {
    Type = "simple";
    ExecStart = "${pkgs.polkit_gnome}/libexec/polkit-gnome-authentication-agent-1";
    Restart = "on-failure";
  };
};
```

Then rebuild: `sudo nixos-rebuild switch`

## Testing Checklist

- [ ] Start app, select profile + bundle, click Apply, close app
- [ ] Reopen app - verify selections are restored
- [ ] Check build log shows "Wrote: /etc/nixos/nixos-toolkit/..." messages
- [ ] Verify files exist in `/etc/nixos/nixos-toolkit/`
- [ ] Run `sudo nixos-rebuild switch` - verify no errors

## Configuration Reminder

Ensure `/etc/nixos/configuration.nix` has the correct import:
```nix
imports = [
  ./nixos-toolkit/state/selected.nix
];
```

## Troubleshooting

### "Unexpected response from helper"
- **Cause**: pkexec can't show password dialog (no polkit agent running)
- **Fix**: Either run as root, or add polkit-gnome to your NixOS config (see above)

### "Cannot run program nixos-toolkit-helper: No such file or directory"
- **Cause**: Helper binary not built or wrong name
- **Fix**: Run `cargo build -p helper` and ensure `NIXOS_TOOLKIT_HELPER` env var is set

### Helper binary location
After building: `/home/gosh/Downloads/NixOSApp-main/target/debug/nixos-toolkit-helper`

## Additional Fixes Found During Testing

### Bug 4: pkexec SHELL Variable Issue
When running from `nix develop`, pkexec rejects commands because the SHELL environment variable points to a nix-wrapped bash not in `/etc/shells`.

**File**: `crates/gui/src/helper/client.rs`

Added `.env_remove("SHELL")` to the Command:
```rust
let mut child = Command::new(cmd)
    .args(args)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::inherit())
    // Remove SHELL env var to avoid pkexec rejecting nix-develop shells
    .env_remove("SHELL")
    .spawn()
```

### Bug 5: State Save Required Second pkexec Call (FIXED)
After apply completed, `save_state()` spawned a **new** privileged helper, requiring a second password prompt. This caused timeouts when no polkit agent was running.

**File**: `crates/gui/src/pages/apply.rs`

**Fix**: Save state using the same helper session that performed the apply, before closing the connection:
```rust
HelperResponse::ApplyComplete { success, message } => {
    // Save state using the same helper session if apply succeeded
    if success {
        if let Some(window) = page.root()... {
            let state = window.get_app_state();
            let ipc_state = state.to_ipc_state();
            client.send(&HelperRequest::WriteState { state: ipc_state })?;
            // Wait for response...
        }
    }
    page.finish_apply(success);
    *client_ref = None;  // Now close the helper
}
```

### Bug 6: Hostname/DNS Conflicts with configuration.nix (FIXED)
When the toolkit set `networking.hostName` or `networking.nameservers`, it would conflict with explicit settings in the user's `configuration.nix`.

**File**: `crates/common/src/nix.rs`

**Fix**: Use `lib.mkDefault` so explicit settings in configuration.nix take precedence:
```rust
// hostname.nix now generates:
networking.hostName = lib.mkDefault "hostname";

// dns.nix now generates:
networking.nameservers = lib.mkDefault [ ... ];
```

## Test Results

**Successfully tested on 2026-01-04:**
- Configuration applied via `nixos-rebuild switch` ✓
- Files written to `/etc/nixos/nixos-toolkit/` ✓
- System rebuilt and units restarted ✓
- State saved using same helper session ✓
- No conflicts with existing configuration.nix settings ✓

## Files Modified (Updated)

| File | Changes |
|------|---------|
| `crates/gui/src/window.rs` | Added imports, `load_state()` method, call from `constructed()` |
| `crates/gui/src/pages/apply.rs` | State save inline in ApplyComplete handler (uses same helper session) |
| `crates/helper/src/commands.rs` | Added `send_log()`, file logging in `apply()`, improved `detect_integration()` |
| `crates/helper/src/nix_gen.rs` | Added write verification in `atomic_write()` |
| `crates/helper/Cargo.toml` | Changed binary name from `helper` to `nixos-toolkit-helper` |
| `crates/gui/src/helper/client.rs` | Added `.env_remove("SHELL")` for pkexec compatibility |
| `crates/common/src/nix.rs` | Use `lib.mkDefault` for hostname and DNS settings |

## Date
2026-01-04

---

# Custom Packages Feature - Implementation

## Date
2026-01-05

## Feature Request
Add ability to paste NixPkg names from search.nixos.org and have the app install and remember them. Support multiple input formats:
- Simple names: `zed-editor`, `htop`
- With prefix: `pkgs.zed-editor`
- Full expressions: `environment.systemPackages = [ pkgs.zed ];`
- Multiple packages: comma, space, or newline separated

## Implementation

### Files Modified

| File | Changes |
|------|---------|
| `crates/common/src/ipc.rs` | Added `custom_packages: Vec<String>` to `AppState` and `Generate`/`Apply` requests |
| `crates/common/src/config.rs` | Added `CUSTOM_PACKAGES_NIX` path constant |
| `crates/common/src/nix.rs` | Added `generate_custom_packages_nix()`, updated `NixGenOptions`, imports in `selected.nix` |
| `crates/gui/src/state.rs` | Added `custom_packages: HashSet<String>` field and helper methods |
| `crates/gui/src/pages/packages.rs` | **NEW FILE** - Custom Packages page UI |
| `crates/gui/src/pages/mod.rs` | Added `mod packages` and `pub use packages::PackagesPage` |
| `crates/gui/src/window.rs` | Added page to navigation sidebar and content stack, sync on state load |
| `crates/gui/src/pages/apply.rs` | Updated preview and apply to include custom_packages |
| `crates/helper/src/nix_gen.rs` | Updated `generate_all_files()` to create `custom-packages.nix` |
| `crates/helper/src/commands.rs` | Updated `generate()` and `apply()` handlers for custom_packages |
| `crates/helper/src/main.rs` | Updated request handlers to pass custom_packages |

### Key Implementation Details

1. **PackagesPage UI** (`packages.rs`):
   - Entry field for pasting package names
   - List of added packages with remove buttons
   - `parse_package_input()` - parses multiple input formats
   - `clean_package_name()` - strips `pkgs.`, `nixpkgs#` prefixes
   - `is_valid_package_name()` - validates package names
   - Duplicate detection against existing bundles

2. **State Persistence**:
   - `custom_packages` stored in `state.json` via IPC
   - Restored on app startup via `sync_from_state()`

3. **Nix Generation**:
   - Creates `/etc/nixos/nixos-toolkit/state/custom-packages.nix`
   - Imported automatically via `selected.nix`

### Generated Nix Output

```nix
# /etc/nixos/nixos-toolkit/state/custom-packages.nix
{ config, lib, pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    zed-editor
    htop
    neofetch
  ];
}
```

## Build Verification

```bash
cargo check  # Completed with only pre-existing warnings
cargo run -p gui  # GUI launches with new Custom Packages page in sidebar
```
