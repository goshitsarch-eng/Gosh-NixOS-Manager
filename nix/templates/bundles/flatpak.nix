# Flatpak Bundle
# Managed by NixOS Toolkit
#
# This bundle enables Flatpak support for installing sandboxed applications.
#
# After applying, add the Flathub repository:
#   flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo

{ config, lib, pkgs, ... }:

{
  # Enable Flatpak service
  services.flatpak.enable = true;

  # XDG portal for Flatpak integration
  xdg.portal = {
    enable = true;
    # Portal backends - these should match your desktop environment
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
  };

  # Flatpak management tools
  environment.systemPackages = with pkgs; [
    flatpak
    gnome-software  # Optional: GUI for managing Flatpaks
  ];
}
