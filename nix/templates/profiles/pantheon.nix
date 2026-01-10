# Pantheon Desktop Profile
# Managed by NixOS Toolkit
#
# Pantheon is the desktop environment from elementary OS.
# It provides a clean, beautiful, and intuitive user experience.

{ config, lib, pkgs, ... }:

{
  # Enable Pantheon desktop environment (Wayland by default)
  services.xserver.enable = true;
  services.xserver.displayManager.lightdm.enable = true;
  services.xserver.desktopManager.pantheon.enable = true;

  # Pantheon-specific services
  services.pantheon.apps.enable = true;

  # XDG portal for Flatpak/sandboxed apps
  xdg.portal = {
    enable = true;
    extraPortals = [
      pkgs.xdg-desktop-portal-gtk
      pkgs.xdg-desktop-portal-pantheon
    ];
  };

  # Elementary/Pantheon apps
  environment.systemPackages = with pkgs; [
    pantheon.elementary-files
    pantheon.elementary-terminal
    pantheon.elementary-code
    pantheon.elementary-photos
    pantheon.elementary-music
    pantheon.elementary-calendar
    pantheon.elementary-camera
    pantheon.elementary-tasks
  ];

  # Enable GVFS for file management features
  services.gvfs.enable = true;
}
