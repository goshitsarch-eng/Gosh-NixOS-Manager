# Sway Desktop Profile
# Managed by NixOS Toolkit
#
# This profile enables Sway, an i3-compatible tiling Wayland compositor
# with a focus on stability and minimalism.

{ config, lib, pkgs, ... }:

{
  # Enable Sway
  programs.sway = {
    enable = true;
    wrapperFeatures.gtk = true;
    extraPackages = with pkgs; [
      swaylock
      swayidle
      swaybg
    ];
  };

  # Display manager
  services.displayManager.sddm = {
    enable = true;
    wayland.enable = true;
  };

  # Essential packages for Sway
  environment.systemPackages = with pkgs; [
    # Core utilities
    waybar          # Status bar
    wofi            # Application launcher (wayland dmenu)
    mako            # Notification daemon
    grim            # Screenshot utility
    slurp           # Screen area selector
    wl-clipboard    # Clipboard utilities
    kanshi          # Auto-configure displays

    # Terminal
    foot
    alacritty

    # File manager
    pcmanfm
    xfce.thunar

    # System utilities
    brightnessctl   # Backlight control
    pamixer         # Audio control
    playerctl       # Media player control
    networkmanagerapplet
    wdisplays       # Display configuration

    # Theme and appearance
    qt5ct
    libsForQt5.qtstyleplugin-kvantum
  ];

  # XDG portal for screen sharing
  xdg.portal = {
    enable = true;
    wlr.enable = true;
    extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
  };

  # Enable dconf for GTK settings
  programs.dconf.enable = true;

  # Polkit authentication agent
  security.polkit.enable = true;

  # Environment variables for Wayland
  environment.sessionVariables = {
    MOZ_ENABLE_WAYLAND = "1";
    QT_QPA_PLATFORM = "wayland";
  };
}
