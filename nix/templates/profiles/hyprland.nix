# Hyprland Desktop Profile
# Managed by NixOS Toolkit
#
# This profile enables Hyprland, a dynamic tiling Wayland compositor
# with smooth animations and modern features.

{ config, lib, pkgs, ... }:

{
  # Enable Hyprland
  programs.hyprland = {
    enable = true;
    xwayland.enable = true;
  };

  # Display manager
  services.displayManager.sddm = {
    enable = true;
    wayland.enable = true;
  };

  # Essential packages for Hyprland
  environment.systemPackages = with pkgs; [
    # Core utilities
    waybar          # Status bar
    wofi            # Application launcher
    dunst           # Notification daemon
    swww            # Wallpaper daemon
    grim            # Screenshot utility
    slurp           # Screen area selector
    wl-clipboard    # Clipboard utilities

    # Terminal
    kitty
    alacritty

    # File manager
    pcmanfm
    xfce.thunar

    # System utilities
    brightnessctl   # Backlight control
    pamixer         # Audio control
    playerctl       # Media player control
    networkmanagerapplet

    # Theme and appearance
    nwg-look        # GTK settings
    qt5ct
    libsForQt5.qtstyleplugin-kvantum
  ];

  # XDG portal for screen sharing and file dialogs
  xdg.portal = {
    enable = true;
    extraPortals = [
      pkgs.xdg-desktop-portal-hyprland
      pkgs.xdg-desktop-portal-gtk
    ];
  };

  # Enable dconf for GTK settings
  programs.dconf.enable = true;

  # Polkit authentication agent
  security.polkit.enable = true;
}
