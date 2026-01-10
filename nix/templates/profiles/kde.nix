# KDE Plasma 6 Desktop Profile
# Managed by NixOS Toolkit
#
# This profile enables the KDE Plasma 6 desktop environment.
# It provides a feature-rich, highly customizable desktop with Wayland by default.
# Note: Plasma 5 is end-of-life and will not be available after NixOS 25.05.

{ config, lib, pkgs, ... }:

{
  # Enable SDDM display manager with Wayland
  services.displayManager.sddm.enable = true;
  services.displayManager.sddm.wayland.enable = true;

  # Enable KDE Plasma 6
  services.desktopManager.plasma6.enable = true;

  # Optional: Uncomment to use X11 session instead of Wayland
  # services.displayManager.defaultSession = "plasmax11";

  # KDE applications
  environment.systemPackages = with pkgs; [
    # Core KDE apps
    kdePackages.kate
    kdePackages.konsole
    kdePackages.dolphin
    kdePackages.ark
    kdePackages.spectacle
    kdePackages.kcalc
    kdePackages.gwenview

    # System utilities
    kdePackages.plasma-systemmonitor
    kdePackages.partitionmanager
    kdePackages.filelight
    kdePackages.kdeconnect-kde
  ];

  # Enable KDE Connect for phone integration
  programs.kdeconnect.enable = true;

  # XDG portal for Flatpak/sandboxed apps
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-kde ];
  };
}
