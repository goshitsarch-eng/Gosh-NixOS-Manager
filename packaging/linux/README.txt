NixOS Toolkit 0.1.0 — Linux Flutter desktop

Extract the complete archive, then run ./nixos-toolkit in this directory.
Keep lib/ and data/ beside the executable. Paths with spaces are supported.
Required runtime: GTK 3, GL/EGL, a graphical X11 or Wayland session, and the
system C/C++ runtime. flutter-build.json and rust-build.txt record the tools.
Build on your oldest supported Linux distribution; a binary built with newer
glibc cannot run on an older distribution merely because GTK is installed.

On other Linux distributions, profiles, packages, forms and preview work.
Apply, generations and maintenance require NixOS, a host-installed
nixos-toolkit-helper, polkit and an authentication agent. Install the host
helper using this repository's NixOS module. Do not run the desktop as root.
The Flatpak also uses the host helper; it contains no privileged service.

Preferences retain $XDG_CONFIG_HOME/nixos-toolkit/preferences.json.
Managed system state remains /etc/nixos/nixos-toolkit/state/state.json.
See BUILDING.md, ARCHITECTURE.md, PLATFORM_SUPPORT.md and MIGRATION_AUDIT.md
in the source repository for build instructions and validation limits.

GPL-3.0-or-later; license included in LICENSE.
https://github.com/goshitsarch-eng/Gosh-NixOS-Manager
