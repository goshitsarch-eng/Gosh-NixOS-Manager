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

# Check if flakes are enabled natively
check_flakes() {
    if nix flake --version &>/dev/null; then
        return 0
    else
        return 1
    fi
}

# Build the correct nix run command based on flakes support
get_nix_run_cmd() {
    if check_flakes; then
        echo "nix run $REPO"
    else
        echo "nix --extra-experimental-features 'nix-command flakes' run $REPO"
    fi
}

# Detect shell config file
detect_shell() {
    case "$SHELL" in
        */zsh) echo ".zshrc" ;;
        */bash) echo ".bashrc" ;;
        */fish) echo ".config/fish/config.fish" ;;
        *) echo ".bashrc" ;;
    esac
}

echo "Checking Nix configuration..."
echo ""

NIX_RUN_CMD=$(get_nix_run_cmd)

if check_flakes; then
    echo "✓ Flakes are enabled natively."
    FLAKES_ENABLED=true
else
    echo "⚠ Flakes are NOT enabled in your Nix configuration."
    echo ""
    echo "The installer will use '--extra-experimental-features' flag."
    echo ""
    echo "To enable flakes permanently, add to /etc/nixos/configuration.nix:"
    echo ""
    echo "  nix.settings.experimental-features = [ \"nix-command\" \"flakes\" ];"
    echo ""
    echo "Then run: sudo nixos-rebuild switch"
    FLAKES_ENABLED=false
fi
echo ""

# Offer to create alias
echo "-----------------------------------"
read -p "Create shell alias '$ALIAS_NAME'? [Y/n] " -n 1 -r < /dev/tty
echo ""
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    SHELL_RC="$HOME/$(detect_shell)"

    # Check if alias already exists
    if grep -qE "(alias|function) $ALIAS_NAME[=' ]" "$SHELL_RC" 2>/dev/null; then
        echo "Alias already exists in $SHELL_RC"
    else
        echo "" >> "$SHELL_RC"
        echo "# NixOS Toolkit" >> "$SHELL_RC"
        if [[ "$SHELL" == */fish ]]; then
            # Fish uses function syntax
            if $FLAKES_ENABLED; then
                echo "function $ALIAS_NAME; nix run $REPO; end" >> "$SHELL_RC"
            else
                echo "function $ALIAS_NAME; nix --extra-experimental-features 'nix-command flakes' run $REPO; end" >> "$SHELL_RC"
            fi
        else
            # Bash/Zsh use alias syntax
            if $FLAKES_ENABLED; then
                echo "alias $ALIAS_NAME='nix run $REPO'" >> "$SHELL_RC"
            else
                echo "alias $ALIAS_NAME='nix --extra-experimental-features \"nix-command flakes\" run $REPO'" >> "$SHELL_RC"
            fi
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

    if $FLAKES_ENABLED; then
        cat > "$DESKTOP_DIR/nixos-toolkit.desktop" << EOF
[Desktop Entry]
Name=NixOS Toolkit
Comment=Configure NixOS with a graphical interface
Exec=nix run $REPO
Icon=preferences-system
Terminal=false
Type=Application
Categories=System;Settings;
Keywords=nixos;configuration;settings;
EOF
    else
        cat > "$DESKTOP_DIR/nixos-toolkit.desktop" << EOF
[Desktop Entry]
Name=NixOS Toolkit
Comment=Configure NixOS with a graphical interface
Exec=nix --extra-experimental-features nix-command flakes run $REPO
Icon=preferences-system
Terminal=false
Type=Application
Categories=System;Settings;
Keywords=nixos;configuration;settings;
EOF
    fi

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
    eval exec "$NIX_RUN_CMD"
else
    echo ""
    echo "To run later, use:"
    echo ""
    echo "  $NIX_RUN_CMD"
    echo ""
    if grep -qE "(alias|function) $ALIAS_NAME[=' ]" "$HOME/$(detect_shell)" 2>/dev/null; then
        echo "Or after reloading your shell:"
        echo ""
        echo "  $ALIAS_NAME"
        echo ""
    fi
fi
