# Gaming Bundle
# Managed by NixOS Toolkit
#
# This bundle enables gaming on NixOS with:
# - Steam with Proton support
# - Lutris for game management
# - Performance tools (MangoHud, Gamemode)
# - Graphics drivers support

{ config, lib, pkgs, ... }:

{
  # Enable Steam
  programs.steam = {
    enable = true;
    remotePlay.openFirewall = true;
    dedicatedServer.openFirewall = true;
    gamescopeSession.enable = true;
  };

  # Enable 32-bit graphics support for games
  hardware.graphics = {
    enable = true;
    enable32Bit = true;
  };

  # Gaming packages
  environment.systemPackages = with pkgs; [
    # Game launchers/managers
    lutris
    heroic                # Epic Games/GOG launcher

    # Performance monitoring
    mangohud
    gamemode

    # Wine for Windows games
    wineWowPackages.stable
    winetricks

    # Utilities
    protonup-qt           # Proton-GE installer
    protontricks

    # Controllers
    antimicrox            # Controller mapping
  ];

  # Enable gamemode
  programs.gamemode.enable = true;

  # Enable gamescope compositor
  programs.gamescope.enable = true;

  # Increase file limits for some games
  security.pam.loginLimits = [
    {
      domain = "*";
      type = "soft";
      item = "nofile";
      value = "1048576";
    }
  ];
}
