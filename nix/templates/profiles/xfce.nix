# XFCE Desktop Profile
# Managed by NixOS Toolkit
#
# XFCE is a lightweight desktop environment that is fast and low on system resources.
# It provides a traditional desktop experience.

{ config, lib, pkgs, ... }:

{
  # Enable X11 windowing system
  services.xserver.enable = true;

  # Enable LightDM display manager
  services.xserver.displayManager.lightdm.enable = true;

  # Enable XFCE desktop environment
  services.xserver.desktopManager.xfce.enable = true;

  # Network Manager applet
  programs.nm-applet.enable = true;

  # XDG portal for Flatpak/sandboxed apps
  xdg.portal = {
    enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
  };

  # XFCE packages
  environment.systemPackages = with pkgs; [
    xfce.xfce4-whiskermenu-plugin
    xfce.xfce4-pulseaudio-plugin
    xfce.xfce4-clipman-plugin
    xfce.xfce4-screenshooter
    xfce.thunar-archive-plugin
    xfce.thunar-volman
  ];
}
