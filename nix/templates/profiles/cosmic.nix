# COSMIC Desktop Profile
# Managed by NixOS Toolkit
#
# COSMIC is System76's modern, Rust-based desktop environment.
# It provides a fast, customizable, and elegant Wayland-native experience.
# Note: COSMIC requires NixOS with COSMIC support (nixos-cosmic overlay or nixpkgs with COSMIC packages).

{ config, lib, pkgs, ... }:

{
  # Enable COSMIC greeter (display manager)
  services.displayManager.cosmic-greeter.enable = true;

  # Enable COSMIC desktop environment
  services.desktopManager.cosmic.enable = true;

  # COSMIC applications and utilities
  environment.systemPackages = with pkgs; [
    # COSMIC core apps (included by default, listed for reference)
    cosmic-files
    cosmic-edit
    cosmic-term
    cosmic-store

    # Additional useful packages
    wl-clipboard  # Wayland clipboard utilities
  ];

  # XDG portal for Flatpak/sandboxed apps
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-cosmic ];
  };

  # Optional: Enable System76 scheduler for better performance
  # Only recommended for System76 hardware
  # services.system76-scheduler.enable = true;

  # Hardware graphics support (required for Wayland)
  hardware.graphics.enable = true;
}
