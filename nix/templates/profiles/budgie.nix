# Budgie Desktop Profile
# Managed by NixOS Toolkit
#
# This profile enables Budgie, a modern desktop focusing on
# simplicity and elegance with GNOME technologies.

{ config, lib, pkgs, ... }:

{
  # Enable X server
  services.xserver.enable = true;

  # Enable LightDM display manager
  services.displayManager.lightdm.enable = true;

  # Enable Budgie desktop environment
  services.xserver.desktopManager.budgie.enable = true;

  # Additional packages
  environment.systemPackages = with pkgs; [
    # Budgie extras
    budgie-backgrounds
    budgie-control-center
    budgie-screensaver

    # Common applications
    gnome-calculator
    gnome-system-monitor
    gnome-disk-utility
    file-roller
    evince  # PDF viewer

    # File manager (Nemo is default but ensure it's present)
    nemo

    # Media
    gnome-photos
    totem  # Video player

    # Utilities
    gnome-screenshot
  ];

  # Enable dconf for GNOME settings
  programs.dconf.enable = true;

  # XDG portal
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
  };

  # Polkit for privilege escalation
  security.polkit.enable = true;
}
