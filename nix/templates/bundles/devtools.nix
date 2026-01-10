# Development Tools Bundle
# Managed by NixOS Toolkit
#
# This bundle provides essential development tools including:
# - Version control (Git, GitHub CLI)
# - Editors (Neovim, VS Code)
# - Languages (Rust, Node.js, Python, Go)
# - Containers (Docker, Podman)

{ config, lib, pkgs, ... }:

{
  # Development packages
  environment.systemPackages = with pkgs; [
    # Version control
    git
    git-lfs
    gh                    # GitHub CLI
    lazygit               # Terminal UI for git

    # Editors
    neovim
    vscode

    # Build tools
    gnumake
    cmake
    ninja
    pkg-config

    # Rust
    rustup

    # Node.js
    nodejs_22
    nodePackages.npm
    nodePackages.pnpm

    # Python
    python3
    python3Packages.pip
    python3Packages.virtualenv

    # Go
    go

    # Utilities
    jq                    # JSON processor
    yq                    # YAML processor
    ripgrep               # Fast grep
    fd                    # Fast find
    httpie                # HTTP client
    curl
    wget

    # Containers
    docker-compose
  ];

  # Enable Git system-wide
  programs.git.enable = true;

  # Enable Docker
  virtualisation.docker = {
    enable = true;
    enableOnBoot = true;
  };

  # Add users to docker group (requires user configuration)
  # users.users.<username>.extraGroups = [ "docker" ];
}
