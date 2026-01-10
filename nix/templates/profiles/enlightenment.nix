# Enlightenment Desktop Profile
# Managed by NixOS Toolkit
#
# This profile enables Enlightenment (E), a unique and visually stunning
# desktop environment with advanced compositing effects.

{ config, lib, pkgs, ... }:

{
  # Enable X server
  services.xserver.enable = true;

  # Enable LightDM display manager
  services.displayManager.lightdm.enable = true;

  # Enable Enlightenment desktop
  services.xserver.desktopManager.enlightenment.enable = true;

  # Additional packages
  environment.systemPackages = with pkgs; [
    # EFL (Enlightenment Foundation Libraries) applications
    enlightenment.terminology  # Terminal emulator
    enlightenment.rage         # Video player
    enlightenment.evisum       # System monitor

    # File manager
    pcmanfm

    # Common applications
    evince       # PDF viewer
    file-roller  # Archive manager
    galculator   # Calculator

    # Media
    mpv
    imv  # Image viewer

    # Utilities
    xfce.xfce4-screenshooter
    networkmanagerapplet
    pavucontrol

    # Themes
    arc-theme
    papirus-icon-theme
  ];

  # Enable dconf for application settings
  programs.dconf.enable = true;

  # XDG portal
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
  };

  # Polkit for privilege escalation
  security.polkit.enable = true;
}
