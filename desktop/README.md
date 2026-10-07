# Flutter Linux frontend

This is the canonical NixOS Toolkit desktop UI. See [BUILDING.md](../BUILDING.md), [ARCHITECTURE.md](../ARCHITECTURE.md) and [MIGRATION_AUDIT.md](../MIGRATION_AUDIT.md) at the repository root.

`flutter build linux` automatically builds and bundles the shared Rust core and templates through `linux/CMakeLists.txt`. For tests, build `desktop-core` at the repository root and set `NIXOS_TOOLKIT_CORE_LIBRARY` to its shared library. There are no generated bridge bindings to maintain.
