# Flutter desktop

This is the NixOS Toolkit UI for native Linux and Flatpak. Start with the root [build guide](../BUILDING.md); the [architecture guide](../ARCHITECTURE.md) explains the Rust core and helper boundary.

`flutter build linux` builds and bundles the core library and Nix templates through `linux/CMakeLists.txt`. For Flutter tests, first build `desktop-core` and set `NIXOS_TOOLKIT_CORE_LIBRARY` to its shared library, as shown in the build guide. There are no generated bridge bindings.

To install the app rather than develop it, use [INSTALLING.md](../INSTALLING.md).
