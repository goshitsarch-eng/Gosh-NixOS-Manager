# i3 Desktop Profile
# Managed by NixOS Toolkit
#
# This profile enables i3, a popular tiling window manager for X11.
# Highly configurable and keyboard-driven.

{ config, lib, pkgs, ... }:

{
  # Enable X server and i3
  services.xserver = {
    enable = true;
    windowManager.i3 = {
      enable = true;
      extraPackages = with pkgs; [
        i3status
        i3lock
        i3blocks
      ];
    };
  };

  # Display manager
  services.displayManager.lightdm.enable = true;

  # Essential packages for i3
  environment.systemPackages = with pkgs; [
    # Core utilities
    dmenu           # Application launcher
    rofi            # Modern dmenu alternative
    dunst           # Notification daemon
    feh             # Wallpaper setter and image viewer
    picom           # Compositor for transparency/effects
    maim            # Screenshot utility
    xclip           # Clipboard utilities
    xdotool         # X11 automation

    # Terminal
    alacritty
    kitty

    # File manager
    pcmanfm
    xfce.thunar

    # System utilities
    arandr          # Display configuration
    lxappearance    # GTK theme switcher
    pavucontrol     # Audio control
    networkmanagerapplet
    pasystray       # PulseAudio systray

    # Fonts
    font-awesome    # Icons for status bar

    # System tray utilities
    blueman
    nm-applet
  ];

  # Enable dconf for application settings
  programs.dconf.enable = true;

  # Polkit authentication agent
  security.polkit.enable = true;
  systemd.user.services.polkit-gnome-authentication-agent-1 = {
    description = "polkit-gnome-authentication-agent-1";
    wantedBy = [ "graphical-session.target" ];
    wants = [ "graphical-session.target" ];
    after = [ "graphical-session.target" ];
    serviceConfig = {
      Type = "simple";
      ExecStart = "${pkgs.polkit_gnome}/libexec/polkit-gnome-authentication-agent-1";
      Restart = "on-failure";
      RestartSec = 1;
      TimeoutStopSec = 10;
    };
  };
}
