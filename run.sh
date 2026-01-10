#!/usr/bin/env bash
# NixOS Toolkit Installer v2
# https://github.com/devjonesafrica/NixOSApp

set -e

REPO="github:devjonesafrica/NixOSApp"
ALIAS_NAME="nixos-toolkit"

printf "===================================\n"
printf "  NixOS Toolkit Installer\n"
printf "===================================\n\n"

# Check if running on NixOS
if [ ! -f /etc/NIXOS ]; then
    printf "Warning: This doesn't appear to be a NixOS system.\n"
    printf "The toolkit is designed for NixOS.\n"
    read -p "Continue anyway? [y/N] " -n 1 -r
    printf "\n"
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check if flakes are enabled
check_flakes() {
    if nix flake --version >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Detect shell config file
detect_shell() {
    case "$SHELL" in
        */zsh) printf ".zshrc" ;;
        */bash) printf ".bashrc" ;;
        */fish) printf ".config/fish/config.fish" ;;
        *) printf ".bashrc" ;;
    esac
}

printf "Checking Nix configuration...\n\n"

if check_flakes; then
    printf "Flakes are enabled. You can run:\n\n"
    printf "  nix run %s\n\n" "$REPO"
    FLAKES_ENABLED=true
else
    printf "Flakes are NOT enabled on your system.\n\n"
    printf "To enable flakes, add this to /etc/nixos/configuration.nix:\n\n"
    printf "  nix.settings.experimental-features = [ \"nix-command\" \"flakes\" ];\n\n"
    printf "Then run: sudo nixos-rebuild switch\n\n"
    printf "For now, you can run with:\n\n"
    printf "  nix --extra-experimental-features 'nix-command flakes' run %s\n\n" "$REPO"
    FLAKES_ENABLED=false
fi

# Offer to create alias
printf -- "-----------------------------------\n"
read -p "Create shell alias '$ALIAS_NAME'? [Y/n] " -n 1 -r
printf "\n"
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    SHELL_RC="$HOME/$(detect_shell)"

    if $FLAKES_ENABLED; then
        ALIAS_CMD="alias $ALIAS_NAME='nix run $REPO'"
    else
        ALIAS_CMD="alias $ALIAS_NAME='nix --extra-experimental-features \"nix-command flakes\" run $REPO'"
    fi

    # Check if alias already exists
    if grep -q "alias $ALIAS_NAME=" "$SHELL_RC" 2>/dev/null; then
        printf "Alias already exists in %s\n" "$SHELL_RC"
    else
        printf "\n" >> "$SHELL_RC"
        printf "# NixOS Toolkit\n" >> "$SHELL_RC"
        printf "%s\n" "$ALIAS_CMD" >> "$SHELL_RC"
        printf "Added alias to %s\n" "$SHELL_RC"
    fi

    printf "\nRun 'source %s' or open a new terminal, then use:\n\n" "$SHELL_RC"
    printf "  %s\n\n" "$ALIAS_NAME"
fi

# Offer to create desktop entry
printf -- "-----------------------------------\n"
read -p "Create desktop entry (application menu)? [Y/n] " -n 1 -r
printf "\n"
if [[ ! $REPLY =~ ^[Nn]$ ]]; then
    DESKTOP_DIR="$HOME/.local/share/applications"
    mkdir -p "$DESKTOP_DIR"

    if $FLAKES_ENABLED; then
        EXEC_CMD="nix run $REPO"
    else
        EXEC_CMD="nix --extra-experimental-features 'nix-command flakes' run $REPO"
    fi

    cat > "$DESKTOP_DIR/nixos-toolkit.desktop" << DESKTOP_EOF
[Desktop Entry]
Name=NixOS Toolkit
Comment=Configure NixOS with a graphical interface
Exec=$EXEC_CMD
Icon=preferences-system
Terminal=false
Type=Application
Categories=System;Settings;
Keywords=nixos;configuration;settings;
DESKTOP_EOF

    printf "Created desktop entry at %s/nixos-toolkit.desktop\n" "$DESKTOP_DIR"
    printf "The app should now appear in your application menu.\n"
fi

printf "\n===================================\n"
printf "  Installation complete!\n"
printf "===================================\n\n"
printf "Quick start:\n"
if $FLAKES_ENABLED; then
    printf "  nix run %s\n" "$REPO"
else
    printf "  nix --extra-experimental-features 'nix-command flakes' run %s\n" "$REPO"
fi
printf "\nOr use the alias: %s\n\n" "$ALIAS_NAME"
