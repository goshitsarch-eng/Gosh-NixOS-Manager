# Container Runtime Bundle
# Managed by NixOS Toolkit
#
# This bundle provides container runtime support with either Docker or Podman.
# Choose one - they can conflict if both are enabled.
#
# Docker: Traditional container runtime (requires daemon)
# Podman: Daemonless, rootless container runtime
#
# After applying Docker, add your user to the 'docker' group:
#   sudo usermod -aG docker $USER

{ config, lib, pkgs, ... }:

{
  # Container packages
  environment.systemPackages = with pkgs; [
    docker-compose     # Multi-container orchestration
    lazydocker         # Terminal UI for Docker
    dive               # Explore Docker image layers
    skopeo             # Container image operations
    buildah            # Build container images
  ];

  # NOTE: Enable ONE of the following in your configuration:

  # Option 1: Docker (traditional, requires daemon)
  # virtualisation.docker = {
  #   enable = true;
  #   enableOnBoot = true;
  #   autoPrune.enable = true;
  # };

  # Option 2: Podman (daemonless, rootless)
  virtualisation.podman = {
    enable = true;
    dockerCompat = true;  # Create docker alias
    defaultNetwork.settings.dns_enabled = true;
  };
}
