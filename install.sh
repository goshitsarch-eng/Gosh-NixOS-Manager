#!/usr/bin/env bash
# Download a signed, prebuilt NixOS Toolkit installer. Run as your normal user.
set -euo pipefail

RELEASES=https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/releases
# Substituted with the repository's public key by the release packager.
PUBLIC_KEY='@CACHE_PUBLIC_KEY@'
VERSION='@RELEASE_VERSION@'

usage() {
    cat <<'EOF'
NixOS Toolkit — prebuilt installation
Usage: bash install.sh [--no-launch] [--flake /etc/nixos#HOST]
                       [--version vVERSION] [--uninstall | --recover]

Installs the app, authenticated helper and menu entry, then opens the app.
Uses /etc/nixos/configuration.nix. Flakes require an explicit local host target
that imports that file. Existing configuration changes are activated too.
Uninstall removes the application and helper; applied system settings remain.
EOF
}

launch=true
action=install
args=()
while (($#)); do
    case "$1" in
        --help|-h) usage; exit 0 ;;
        --no-launch) launch=false ;;
        --version) [[ $# -ge 2 ]] || { usage; exit 1; }; VERSION=$2; shift ;;
        --flake) [[ $# -ge 2 ]] || { usage; exit 1; }; args+=(--flake "$2"); shift ;;
        --uninstall) action=uninstall; launch=false ;;
        --recover) action=recover; launch=false ;;
        *) echo "Unknown option: $1" >&2; usage; exit 1 ;;
    esac
    shift
done
[[ $EUID -ne 0 ]] || { echo 'Run this as your normal desktop user; it asks for sudo itself.' >&2; exit 1; }
[[ -f /etc/NIXOS ]] || { echo 'This installer requires NixOS.' >&2; exit 1; }
for command in curl nix-store sudo mktemp; do
    command -v "$command" >/dev/null || { echo "Required command unavailable: $command" >&2; exit 1; }
done
case "$(uname -m)" in
    x86_64) system=x86_64-linux ;;
    *) echo 'Prebuilt installation currently supports x86_64 NixOS only.' >&2; exit 1 ;;
esac
if [[ -f /etc/nixos/flake.nix && ${#args[@]} -eq 0 && $action == install ]]; then
    echo 'This host uses flakes. Rerun with --flake /etc/nixos#YOUR_HOST.' >&2
    echo 'The selected host must import /etc/nixos/configuration.nix.' >&2
    exit 1
fi

# A checkout can bootstrap the published release, using the checked-in key.
if [[ $PUBLIC_KEY == '@CACHE_PUBLIC_KEY@' ]]; then
    script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
    [[ -f $script_dir/packaging/nix/cache-public-key ]] || {
        echo 'Use install.sh from a published GitHub release, or a complete checkout.' >&2; exit 1;
    }
    PUBLIC_KEY=$(<"$script_dir/packaging/nix/cache-public-key")
fi
[[ $VERSION == '@RELEASE_VERSION@' ]] && VERSION=latest
if [[ $VERSION == latest ]]; then
    manifest_url="$RELEASES/latest/download/release-$system.txt"
elif [[ $VERSION =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$ ]]; then
    manifest_url="$RELEASES/download/$VERSION/release-$system.txt"
else
    echo 'Invalid release version.' >&2; exit 1
fi

umask 077
temporary=$(mktemp -d)
trap 'rm -rf -- "$temporary"' EXIT
echo 'Finding the prebuilt NixOS Toolkit release…'
curl --fail --silent --show-error --location --proto '=https' --proto-redir '=https' \
    --retry 3 --connect-timeout 15 --max-time 120 "$manifest_url" -o "$temporary/release"
mapfile -t manifest < "$temporary/release"
[[ ${#manifest[@]} -eq 4 && ${manifest[0]} == nixos-toolkit-release-v1 && \
   ${manifest[1]} =~ ^v[0-9]+\.[0-9]+\.[0-9]+([.-][A-Za-z0-9.-]+)?$ && \
   ${manifest[2]} == "$system" && \
   ${manifest[3]} =~ ^/nix/store/[0-9a-z]{32}-nixos-toolkit-installer-[0-9A-Za-z.+-]+$ ]] || {
    echo 'Invalid release manifest; nothing was installed.' >&2; exit 1;
}
[[ $VERSION == latest || $VERSION == "${manifest[1]}" ]] || {
    echo 'Release manifest version mismatch.' >&2; exit 1;
}
version=${manifest[1]}
installer=${manifest[3]}
cache="$RELEASES/download/$version"
echo "NixOS Toolkit $version: $action"
echo 'The app and helper are prebuilt. NixOS will evaluate and activate your system configuration.'
echo 'Your configuration is backed up before changes. The desktop app runs without root privileges.'
sudo -v
# Trust is scoped to this download, not added permanently to nix.conf. Explicit
# output paths and disabled builders prevent an unavailable app falling back to
# compiling a toolchain on the user's computer.
sudo nix-store --realise "$installer" --option substituters "https://cache.nixos.org $cache" \
    --option extra-trusted-public-keys "$PUBLIC_KEY" --option require-sigs true \
    --option max-jobs 0 --option builders '' --add-root "$temporary/installer" --indirect
sudo "$installer/bin/nixos-toolkit-install" "$action" "${args[@]}"
if $launch; then
    if [[ -n ${DISPLAY:-}${WAYLAND_DISPLAY:-} ]]; then
        echo 'Opening NixOS Toolkit…'
        log_dir=${XDG_STATE_HOME:-$HOME/.local/state}/nixos-toolkit
        mkdir -p "$log_dir"
        nohup /run/current-system/sw/bin/nixos-toolkit >"$log_dir/launcher.log" 2>&1 </dev/null &
    else
        echo 'Installed. Open NixOS Toolkit from your desktop application menu.'
    fi
fi
