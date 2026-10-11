# VirtualBox Bundle
# Managed by NixOS Toolkit
#
# This bundle enables Oracle VirtualBox with Extension Pack.
# Note: VirtualBox Extension Pack is non-free software.
#
# After applying, add your user to the 'vboxusers' group:
#   sudo usermod -aG vboxusers $USER

{ config, lib, pkgs, ... }:

{
  # Allow unfree packages (required for extension pack)
  nixpkgs.config.allowUnfree = true;

  # Enable VirtualBox host
  virtualisation.virtualbox.host = {
    enable = true;
    enableExtensionPack = true;  # Non-free extension pack
  };

  # VirtualBox packages
  environment.systemPackages = with pkgs; [
    virtualbox
  ];
}
