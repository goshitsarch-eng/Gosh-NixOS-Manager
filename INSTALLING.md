# Install the prebuilt NixOS app

On an x86_64 NixOS desktop, run this as your normal user:

```sh
curl -fsSL https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/releases/latest/download/install.sh | bash
```

The installer asks for sudo, downloads signed prebuilt packages, installs the
app and authenticated helper, adds the application-menu entry, and launches the
app as your normal user. The app itself may ask for normal polkit authentication
when it reads or changes system configuration. Installation does not grant
passwordless access to the helper.

Downloads use a signed Nix binary cache hosted as GitHub release assets. Only
missing runtime dependencies are downloaded. The app, Rust core, Flutter runtime,
and helper are not compiled on your computer. NixOS still evaluates/builds the
system integration and activates a generation. Existing unapplied configuration
changes are included in that activation. Keep your configuration working before
installing; installation does not update channels or flake locks.

To inspect the script first:

```sh
curl -fsSL https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/releases/latest/download/install.sh -o install-toolkit.sh
less install-toolkit.sh
bash install-toolkit.sh
```

Use `--no-launch` on a terminal-only host, or `--version v0.1.0` to choose an
available release. Running the installer again updates its existing module.
The first release supports x86_64; ARM64 binaries have not been validated.

## Configuration and flakes

Automatic integration supports a regular `/etc/nixos/configuration.nix` with
a conventional module body and a literal `imports = [ ... ];` list (or no imports
yet). It adds a marked import of `/etc/nixos/nixos-toolkit-install.nix` and, if
needed, the Toolkit selections import. It preserves your other configuration.
The installer module contains the exact downloaded app/helper store paths and
the NixOS 25.05 activation compatibility fix.

For a flake host that imports this conventional configuration file, explicitly
select the host:

```sh
bash install-toolkit.sh --flake /etc/nixos#YOUR_HOST
```

New files in a Git flake are marked intent-to-add so evaluation can see them.
Review and commit the configuration changes normally; the installer never makes
a Git commit. More complex module expressions, symlinked configurations, remote
flakes, and configurations outside `/etc/nixos` need manual integration through
the repository's NixOS module. The installer stops rather than guessing which
host or file to edit. Remove a previous manual Toolkit package/module declaration
before switching installation methods, to avoid duplicate packages or options.

Keep manual packages and settings outside `/etc/nixos/nixos-toolkit/`. The app
regenerates that directory. The installer owns `nixos-toolkit-install.nix`; do
not hand-edit it. Your existing desktop/software selections are preserved.

## Recovery and uninstall

Private backups are under `/var/lib/nixos-toolkit-installer/backups/`. Failed
builds restore the edited files; failed activation also attempts to restore the
previous running and boot systems. If power loss or interruption leaves a
pending installation, run:

```sh
bash install-toolkit.sh --recover
```

Recovery refuses to overwrite files changed by another editor after the
installer wrote them. Its private backup remains available for manual recovery.
System rollback does not restore application data. You can also select the
previous NixOS generation from the boot menu.

To remove the application and helper:

```sh
bash install-toolkit.sh --uninstall
```

Uninstall retains generated selections and their import so it does not remove
your configured desktop, applications or services. It removes only the installer
module and its marked import, then activates the resulting configuration.

## Maintaining releases

`nix build .#installer` builds the app, matching helper, and small privileged
installer. `scripts/package-nix-release.py` creates a flat, signed Nix cache:

```sh
python3 scripts/package-nix-release.py --version v0.1.0 \
  --signing-key /private/location/cache-secret-key \
  --output dist/nix-release-v0.1.0
```

The public key is pinned in `packaging/nix/cache-public-key` and embedded in
published installer scripts. The private key must remain outside the repository
and artifacts. Back it up securely. Cache trust is scoped to the install command;
the installer does not permanently change Nix settings or disable signatures.

Each release contains `install.sh`, an architecture manifest, signed `.narinfo`
metadata, compressed store objects, checksums, and build information. GitHub
serves these files as an ordinary HTTP Nix cache. A release must have every asset
uploaded and verified before publication, and published assets must not be
overwritten. Cachix or another HTTP cache can be used later without changing
the Nix packages. Hosted CI results and local validation are recorded separately.
