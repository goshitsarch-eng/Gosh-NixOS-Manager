NixOS Toolkit — Linux desktop archive

On NixOS, use the prebuilt Nix installer:
https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/blob/main/INSTALLING.md

For this archive, extract everything and run ./nixos-toolkit in a graphical
session. Keep lib/ and data/ beside the executable. Paths with spaces work.
You need GTK 3, GL/EGL, X11 or Wayland, and the system C/C++ runtime. A build
made with a newer glibc may not run on an older distribution. Tool versions
are recorded in flutter-build.json and rust-build.txt.

Configuration previews work on other Linux distributions. Apply, generations,
and maintenance require NixOS, a matching host helper, polkit, and a desktop
authentication agent. The desktop app runs as your normal user.

User preferences: $XDG_CONFIG_HOME/nixos-toolkit/preferences.json
(default base: ~/.config).
System selections: /etc/nixos/nixos-toolkit/state/state.json.
Keep manual settings outside /etc/nixos/nixos-toolkit/.

Usage, build instructions, and tested platforms:
https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/tree/main/docs

GPL-3.0-or-later. See the included LICENSE.
