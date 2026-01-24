# Cinnamon Desktop Profile
# Managed by NixOS Toolkit
#
# Cinnamon is a desktop environment from Linux Mint.
# It provides a traditional desktop experience with modern features.

{ config, lib, pkgs, ... }:

{
  # Enable X11 windowing system
  services.xserver.enable = true;

  # Disable other display managers
  services.displayManager.gdm.enable = lib.mkForce false;
  services.displayManager.sddm.enable = lib.mkForce false;

  # Enable LightDM display manager
  services.xserver.displayManager.lightdm.enable = lib.mkForce true;

  # Enable Cinnamon desktop environment
  services.xserver.desktopManager.cinnamon.enable = true;

  # Cinnamon packages
  environment.systemPackages = with pkgs; [
    cinnamon.nemo-with-extensions
    cinnamon.cinnamon-screensaver
    cinnamon.cinnamon-control-center
    gnome-screenshot
    gnome-calculator
    gnome-system-monitor
  ];
}
