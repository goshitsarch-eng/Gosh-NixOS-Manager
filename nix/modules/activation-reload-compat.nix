# NixOS 25.05 submits other reloads while dbus-daemon is still reloading.
# NSS lookup delays can then consume the synchronous D-Bus call timeout.
{ config, lib, ... }:
let
  cfg = config.programs.nixos-toolkit;
in
{
  options.programs.nixos-toolkit.serializeDbusReload = lib.mkOption {
    type = lib.types.bool;
    default = false;
    description = ''
      Wait for the D-Bus service reload job before submitting other unit reloads.
      This compatibility patch targets the NixOS 25.05 activation program and
      preserves its normal service failure reporting.
    '';
  };

  config = lib.mkIf cfg.serializeDbusReload {
    assertions = [{
      assertion = config.system.nixos.release == "25.05";
      message = "The toolkit D-Bus reload compatibility patch targets NixOS 25.05 only.";
    }];
    nixpkgs.overlays = [
      (_final: prev: {
        switch-to-configuration-ng = prev.switch-to-configuration-ng.overrideAttrs (old: {
          patches = (old.patches or [ ]) ++ [ ../patches/serialize-dbus-reload.patch ];
        });
      })
    ];
  };
}
