# MATE Desktop Profile
# Managed by NixOS Toolkit
#
# MATE is a fork of GNOME 2, providing an intuitive and attractive desktop
# using traditional metaphors. It's lightweight and efficient.

{ config, lib, pkgs, ... }:

{
  # Enable X11 windowing system
  services.xserver.enable = true;

  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;

  # Enable LightDM display manager
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;

  # Enable MATE desktop environment
  services.xserver.desktopManager.mate.enable = true;

  # Network Manager applet
  programs.nm-applet.enable = true;

  # XDG portal
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
  };

  # MATE packages
  environment.systemPackages = with pkgs; [
    mate.mate-utils
    mate.mate-media
    mate.mate-power-manager
    mate.engrampa
    mate.pluma
    mate.atril
  ];
}
