# Virtualization Bundle (KVM/QEMU)
# Managed by NixOS Toolkit
#
# This bundle enables KVM virtualization with libvirt and virt-manager.
# It provides fast, hardware-accelerated virtual machines.
#
# Note: After applying, add your user to the 'libvirtd' group:
#   sudo usermod -aG libvirtd $USER

{ config, lib, pkgs, ... }:

{
  # Enable KVM kernel modules
  boot.kernelModules = [ "kvm-amd" "kvm-intel" ];

  # Enable libvirtd daemon
  virtualisation.libvirtd = {
    enable = true;
    qemu = {
      package = pkgs.qemu_kvm;
      runAsRoot = true;
      swtpm.enable = true;  # TPM emulation for Windows 11
      ovmf = {
        enable = true;
        packages = [ pkgs.OVMFFull.fd ];
      };
    };
  };

  # Virtualization packages
  environment.systemPackages = with pkgs; [
    virt-manager        # GUI for managing VMs
    virt-viewer         # VM console viewer
    qemu                # QEMU emulator
    OVMF                # UEFI firmware
    spice-gtk           # SPICE client
    win-virtio          # Windows VirtIO drivers
  ];

  # Enable spice USB redirection
  virtualisation.spiceUSBRedirection.enable = true;
}
