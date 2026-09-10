# Documentation

These files describe the **current** tree (libcosmic GUI, privileged helper, flake packages). They were rewritten against the source; do not treat older README drafts or `docs/migration/` as present tense.

| File | Audience | Contents |
|------|----------|----------|
| [../README.md](../README.md) | Users | Install, setup, features, apply vs preview |
| [architecture.md](architecture.md) | Contributors | Crates, process model, IPC, Nix generation, state |
| [development.md](development.md) | Contributors | Build, test, CI, adding profiles/bundles, env vars |
| [reference.md](reference.md) | Contributors | Paths, IPC variants, catalogs, helper stubs, limitations |
| [migration/](migration/) | Archaeology | GTK4 → libcosmic design notes. **Historical.** |

Identity:

- Repository: `goshitsarch-eng/Gosh-NixOS-Manager`
- Product name: NixOS Toolkit
- App ID: `io.github.goshitsarch_eng.NixosToolkit`
- GUI binary: `nixos-toolkit` (Cargo package `gui`)
- Helper binary: `nixos-toolkit-helper` (Cargo package `helper`)
