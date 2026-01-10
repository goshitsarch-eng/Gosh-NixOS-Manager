# Communication Bundle
# Managed by NixOS Toolkit
#
# Chat, video calls, and messaging applications.

{ config, lib, pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    # Instant messaging
    discord
    signal-desktop
    element-desktop
    telegram-desktop

    # Work communication
    slack
    zoom-us

    # Email (if not using DE default)
    thunderbird

    # IRC/Matrix
    hexchat
    nheko

    # Video conferencing
    jitsi-meet
  ];
}
