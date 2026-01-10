# Security Tools Bundle
# Managed by NixOS Toolkit
#
# Password managers, encryption, and security utilities.

{ config, lib, pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    # Password managers
    keepassxc
    bitwarden-desktop

    # Encryption
    veracrypt
    age
    sops

    # GPG
    gnupg
    pinentry-curses
    pinentry-qt

    # Security utilities
    pass           # Unix password manager
    gopass         # Go-based pass alternative
    pwgen          # Password generator

    # Network security
    wireshark
    nmap

    # File encryption
    gocryptfs
    cryptsetup
  ];

  # Enable GnuPG agent
  programs.gnupg.agent = {
    enable = true;
    enableSSHSupport = true;
  };
}
