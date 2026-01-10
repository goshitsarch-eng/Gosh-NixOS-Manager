#!/usr/bin/env bash
# NixOS Toolkit Installer
# https://github.com/devjonesafrica/NixOSApp

set -e

REPO="github:devjonesafrica/NixOSApp"
ALIAS_NAME="nixos-toolkit"

echo "==================================="
echo "  NixOS Toolkit Installer"
echo "==================================="
echo ""

# Check if running on NixOS
if [ ! -f /etc/NIXOS ]; then
    echo "Warning: This doesn't appear to be a NixOS system."
    echo "The toolkit is designed for NixOS."
    read -p "Continue anyway? [y/N] " -n 1 -r
    echo ""
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check if flakes are enabled
check_flakes() {
    if nix flake --version &>/dev/null; then
        return 0
    else
        return 1
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

if check_flakes; then
    echo "Flakes are enabled. You can run:"
    echo ""
    echo "  nix run $REPO"
    echo ""
    FLAKES_ENABLED=true
else
    echo "Flakes are NOT enabled on your system."
    echo ""
    echo "To enable flakes, add this to /etc/nixos/configuration.nix:"
    echo ""
    echo "  nix.settings.experimental-features = [ \"nix-command\" \"flakes\" ];"
    echo ""
    echo "Then run: sudo nixos-rebuild switch"
    echo ""
    echo "For now, you can run with:"
    echo ""
    echo "  nix --extra-experimental-features 'nix-command flakes' run $REPO"
    echo ""
    FLAKES_ENABLED=false
fi

# Offer to create alias
echo "-----------------------------------"
read -p "Create shell alias '$ALIAS_NAME'? [Y/n] " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    SHELL_RC="$HOME/$(detect_shell)"

    if $FLAKES_ENABLED; then
        ALIAS_CMD="alias $ALIAS_NAME='nix run $REPO'"
    else
        ALIAS_CMD="alias $ALIAS_NAME='nix --extra-experimental-features \"nix-command flakes\" run $REPO'"
    fi

    # Check if alias already exists
    if grep -q "alias $ALIAS_NAME=" "$SHELL_RC" 2>/dev/null; then
        echo "Alias already exists in $SHELL_RC"
    else
        echo "" >> "$SHELL_RC"
        echo "# NixOS Toolkit" >> "$SHELL_RC"
        echo "$ALIAS_CMD" >> "$SHELL_RC"
        echo "Added alias to $SHELL_RC"
    fi

    echo ""
    echo "Run 'source $SHELL_RC' or open a new terminal, then use:"
    echo ""
    echo "  $ALIAS_NAME"
    echo ""
fi

# Offer to create desktop entry
echo "-----------------------------------"
read -p "Create desktop entry (application menu)? [Y/n] " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    DESKTOP_DIR="$HOME/.local/share/applications"
    mkdir -p "$DESKTOP_DIR"

    if $FLAKES_ENABLED; then
        EXEC_CMD="nix run $REPO"
    else
        EXEC_CMD="nix --extra-experimental-features 'nix-command flakes' run $REPO"
    fi

    cat > "$DESKTOP_DIR/nixos-toolkit.desktop" << EOF
[Desktop Entry]
Name=NixOS Toolkit
Comment=Configure NixOS with a graphical interface
Exec=$EXEC_CMD
Icon=preferences-system
Terminal=false
Type=Application
Categories=System;Settings;
Keywords=nixos;configuration;settings;
EOF

    echo "Created desktop entry at $DESKTOP_DIR/nixos-toolkit.desktop"
    echo "The app should now appear in your application menu."
fi

echo ""
echo "==================================="
echo "  Installation complete!"
echo "==================================="
echo ""
echo "Quick start:"
if $FLAKES_ENABLED; then
    echo "  nix run $REPO"
else
    echo "  nix --extra-experimental-features 'nix-command flakes' run $REPO"
fi
echo ""
echo "Or use the alias: $ALIAS_NAME"
echo ""
