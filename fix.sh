#!/usr/bin/env bash
# NixOS Toolkit Quick Installer
set -e

REPO="github:devjonesafrica/NixOSApp"

echo "==================================="
echo "  NixOS Toolkit Installer"
echo "==================================="

# Always use experimental features flag (works whether flakes enabled or not)
CMD="nix --extra-experimental-features 'nix-command flakes' run $REPO"

echo ""
echo "To enable flakes permanently, add to /etc/nixos/configuration.nix:"
echo "  nix.settings.experimental-features = [ \"nix-command\" \"flakes\" ];"
echo ""
echo "Run the app with:"
echo "  $CMD"

echo ""
echo "-----------------------------------"
echo "Create alias 'nixos-toolkit'? [Y/n]"
read REPLY

if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    if [ -f "$HOME/.zshrc" ]; then
        RC="$HOME/.zshrc"
    else
        RC="$HOME/.bashrc"
    fi

    # Remove old alias if exists, then add new one
    if grep -q "alias nixos-toolkit=" "$RC" 2>/dev/null; then
        sed -i '/alias nixos-toolkit=/d' "$RC"
        echo "Updated existing alias in $RC"
    fi

    echo "" >> "$RC"
    echo "# NixOS Toolkit" >> "$RC"
    echo "alias nixos-toolkit='$CMD'" >> "$RC"
    echo "Added to $RC - restart terminal or run: source $RC"
fi

echo ""
echo "Done! Run: nixos-toolkit"
