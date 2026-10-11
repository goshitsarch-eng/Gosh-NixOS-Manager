{ pkgs, gui, helper, version }:
pkgs.runCommand "nixos-toolkit-installer-${version}" { } ''
  mkdir -p $out/bin $out/share/nixos-toolkit-installer/nix
  cp ${./install.py} $out/share/nixos-toolkit-installer/install.py
  cp -r ${../../nix/modules} $out/share/nixos-toolkit-installer/nix/modules
  cp -r ${../../nix/patches} $out/share/nixos-toolkit-installer/nix/patches
  cat > $out/share/nixos-toolkit-installer/packages.json <<EOF
  {"gui":"${gui}","helper":"${helper}","version":"${version}"}
  EOF
  cat > $out/bin/nixos-toolkit-install <<EOF
  #!${pkgs.runtimeShell}
  export PATH=/run/current-system/sw/bin:/run/wrappers/bin
  exec ${pkgs.python3}/bin/python3 $out/share/nixos-toolkit-installer/install.py "\$@"
  EOF
  chmod +x $out/bin/nixos-toolkit-install
''
