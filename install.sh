#!/usr/bin/env bash
# NixOS Toolkit Installer
# https://github.com/goshitsarch-eng/Gosh-NixOS-Manager

set -e

REPO="github:goshitsarch-eng/Gosh-NixOS-Manager"
ALIAS_NAME="nixos-toolkit"

echo "==================================="
echo "  NixOS Toolkit Installer"
echo "==================================="
echo ""

# Check if running on NixOS
if [ ! -f /etc/NIXOS ]; then
    echo "Warning: This doesn't appear to be a NixOS system."
    echo "The toolkit is designed for NixOS."
    read -p "Continue anyway? [y/N] " -n 1 -r < /dev/tty
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Explicit feature flags also work when already enabled in the host config.
# `nix flake --version` does not establish that either feature is enabled.
NIX_RUN_CMD="nix --extra-experimental-features 'nix-command flakes' run $REPO"

# Detect shell config file
detect_shell() {
    case "${SHELL:-}" in
        */zsh) echo ".zshrc" ;;
        */bash) echo ".bashrc" ;;
        */fish) echo ".config/fish/config.fish" ;;
        *) echo ".bashrc" ;;
    esac
}

echo "The launcher enables nix-command and flakes for this command only."
echo "No system Nix settings will be changed."
echo ""

# Offer to create alias
echo "-----------------------------------"
read -p "Create shell alias '$ALIAS_NAME'? [Y/n] " -n 1 -r < /dev/tty
echo ""
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    SHELL_RC="$HOME/$(detect_shell)"

    # Check if alias already exists
    if grep -qE "(alias|function) $ALIAS_NAME[=;[:space:]]" "$SHELL_RC" 2>/dev/null; then
        echo "Alias already exists in $SHELL_RC"
    else
        mkdir -p "$(dirname "$SHELL_RC")"
        echo "" >> "$SHELL_RC"
        echo "# NixOS Toolkit" >> "$SHELL_RC"
        if [[ "${SHELL:-}" == */fish ]]; then
            echo "function $ALIAS_NAME; nix --extra-experimental-features 'nix-command flakes' run $REPO; end" >> "$SHELL_RC"
        else
            echo "alias $ALIAS_NAME='nix --extra-experimental-features \"nix-command flakes\" run $REPO'" >> "$SHELL_RC"
        fi
        echo "✓ Added alias to $SHELL_RC"
    fi
    echo ""
fi

# Offer to create desktop entry
echo "-----------------------------------"
read -p "Create desktop entry (application menu)? [Y/n] " -n 1 -r < /dev/tty
echo ""
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    DESKTOP_DIR="$HOME/.local/share/applications"
    mkdir -p "$DESKTOP_DIR"

    cat > "$DESKTOP_DIR/nixos-toolkit.desktop" << EOF
[Desktop Entry]
Name=NixOS Toolkit
Comment=Configure NixOS with a graphical interface
Exec=nix --extra-experimental-features "nix-command flakes" run $REPO
Icon=preferences-system
Terminal=false
Type=Application
Categories=System;Settings;
Keywords=nixos;configuration;settings;
EOF

    echo "✓ Created desktop entry at $DESKTOP_DIR/nixos-toolkit.desktop"
    echo "The app should now appear in your application menu."
fi

echo ""
echo "==================================="
echo "  Installation complete!"
echo "==================================="
echo ""

# Offer to run immediately
echo "-----------------------------------"
read -p "Run NixOS Toolkit now? [Y/n] " -n 1 -r < /dev/tty
echo ""
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    echo ""
    echo "Starting NixOS Toolkit..."
    echo "Command: $NIX_RUN_CMD"
    echo ""
    exec nix --extra-experimental-features "nix-command flakes" run "$REPO"
else
    echo ""
    echo "To run later, use:"
    echo ""
    echo "  $NIX_RUN_CMD"
    echo ""
    if grep -qE "(alias|function) $ALIAS_NAME[=;[:space:]]" "$HOME/$(detect_shell)" 2>/dev/null; then
        echo "Or after reloading your shell:"
        echo ""
        echo "  $ALIAS_NAME"
        echo ""
    fi
fi
