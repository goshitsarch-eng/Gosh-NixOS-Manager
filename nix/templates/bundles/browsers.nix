# Web Browsers Bundle
# Managed by NixOS Toolkit
#
# Additional web browsers with different privacy/feature focuses.

{ config, lib, pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    # Mozilla Firefox
    firefox

    # Chromium-based
    chromium
    brave
    ungoogled-chromium

    # Privacy-focused
    tor-browser

    # Minimal/alternative
    qutebrowser
    nyxt
  ];

  # Firefox policies (optional hardening)
  programs.firefox = {
    enable = true;
    policies = {
      DisableTelemetry = true;
      DisableFirefoxStudies = true;
      EnableTrackingProtection = {
        Value = true;
        Locked = true;
        Cryptomining = true;
        Fingerprinting = true;
      };
    };
  };
}
