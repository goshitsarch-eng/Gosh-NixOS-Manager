# System Utilities Bundle
# Managed by NixOS Toolkit
#
# Helpful command-line and system tools.

{ config, lib, pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    # System monitoring
    htop
    btop
    iotop
    nethogs

    # System info
    neofetch
    fastfetch
    inxi
    lshw
    pciutils
    usbutils

    # File utilities
    tree
    ncdu      # Disk usage analyzer
    duf       # Disk usage/free utility
    eza       # Modern ls replacement
    bat       # Cat with syntax highlighting
    fd        # Find alternative
    ripgrep   # Grep alternative
    fzf       # Fuzzy finder

    # Archive utilities
    unzip
    unrar
    p7zip
    atool

    # Network utilities
    wget
    curl
    aria2     # Download manager
    rsync

    # Text processing
    jq        # JSON processor
    yq        # YAML processor

    # Terminal multiplexer
    tmux
    screen

    # Misc utilities
    tldr      # Simplified man pages
    tealdeer  # Rust tldr client
  ];
}
