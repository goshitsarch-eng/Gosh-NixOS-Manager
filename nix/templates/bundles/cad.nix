# 3D & CAD Bundle
# Managed by NixOS Toolkit
#
# 3D modeling, CAD, and design software.

{ config, lib, pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    # 3D Modeling
    blender

    # CAD
    freecad
    openscad
    librecad  # 2D CAD

    # Electronics CAD
    kicad

    # Mesh processing
    meshlab

    # Slicers (3D printing)
    prusa-slicer
    cura

    # Image to 3D
    makehuman
  ];

  # Enable OpenGL for 3D applications
  hardware.graphics.enable = true;
}
