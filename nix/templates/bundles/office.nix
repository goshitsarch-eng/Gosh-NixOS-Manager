# Office/Productivity Bundle
# Managed by NixOS Toolkit
#
# This bundle provides office and productivity applications.

{ config, lib, pkgs, ... }:

{
  environment.systemPackages = with pkgs; [
    # Office suite
    libreoffice-fresh
    onlyoffice-bin     # Alternative office suite

    # PDF
    evince             # PDF viewer
    pdfarranger        # PDF manipulation
    ocrmypdf           # OCR for PDFs

    # Note taking
    obsidian
    logseq
    xournalpp          # Handwritten notes

    # Email
    thunderbird

    # Calendar/Tasks
    gnome-calendar
    endeavour          # Task manager

    # Communication
    signal-desktop
    element-desktop    # Matrix client

    # Finance
    gnucash

    # Scanning
    simple-scan
  ];

  # Printer support
  services.printing.enable = true;
  services.printing.drivers = with pkgs; [
    gutenprint
    hplip
  ];

  # Scanner support
  hardware.sane.enable = true;
}
