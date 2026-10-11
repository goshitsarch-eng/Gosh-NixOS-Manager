# LXQt Desktop Profile
# Managed by NixOS Toolkit
#
# This profile enables LXQt, a lightweight Qt-based desktop environment.
# Great for older hardware or users who prefer a lightweight system.

{ config, lib, pkgs, ... }:

{
  # Enable X server
  services.xserver.enable = true;

  # Disable other display managers
  services.xserver.displayManager.gdm.enable = lib.mkForce false;
  services.xserver.displayManager.lightdm.enable = lib.mkForce false;

  # Enable SDDM display manager (Qt-based, matches LXQt)
  services.displayManager.sddm.enable = lib.mkForce true;

  # Enable LXQt desktop environment
  services.xserver.desktopManager.lxqt.enable = true;

  # Additional packages
  environment.systemPackages = with pkgs; [
    # LXQt utilities
    lxqt.lxqt-archiver
    lxqt.lxqt-sudo
    lxqt.pavucontrol-qt

    # File manager
    lxqt.pcmanfm-qt

    # Terminal
    lxqt.qterminal

    # Applications
    lxqt.qps  # Qt Process manager
    featherpad  # Qt text editor
    lxqt.lximage-qt  # Image viewer

    # Qt theming
    libsForQt5.qt5ct
    libsForQt5.qtstyleplugin-kvantum

    # System utilities
    xarchiver
    galculator
  ];

  # Enable dconf for application settings
  programs.dconf.enable = true;

  # XDG portal
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.lxqt.xdg-desktop-portal-lxqt ];
  };

  # Polkit for privilege escalation
  security.polkit.enable = true;
}
