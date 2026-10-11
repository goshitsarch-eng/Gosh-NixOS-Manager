# Software bundle package audit

Verified against upstream documentation and Nixpkgs on 2026-10-11. The Rust
catalog in `crates/common/src/actions.rs` is authoritative for both frontends
and the helper. Enabling a bundle selects its installable entries; manual setup
entries stay unchecked and link to the upstream project.

## Flatpak

| Entry | Nix package | Source / behavior |
| --- | --- | --- |
| Flatpak | `flatpak` | [Flatpak](https://flatpak.org/setup/NixOS): enables the service and desktop portals. |
| Bazaar App Store | `bazaar` | [Bazaar](https://usebazaar.org/), [Flathub](https://flathub.org/apps/io.github.kolunmi.Bazaar). Selecting Bazaar alone also enables Flatpak and portals. This is the app store, not the Bazaar version-control tool. |

Flathub remote setup remains a user choice in Flatpak/Bazaar. The bundle does
not execute remote installers or change another user's Flatpak remotes.

## AI Tools

| Requested tool | Managed package / status | Verified source |
| --- | --- | --- |
| ChatGPT Desktop | Manual setup; Nixpkgs `chatgpt` targets Darwin and must not be emitted for Linux. An official Linux package has not been integrated here. | [OpenAI download](https://chatgpt.com/download/) |
| Hermes Agent Desktop | Manual setup; upstream's Nix installation path is best effort. Its desktop integration has not been validated here. | [Hermes Desktop](https://hermes-agent.nousresearch.com/docs/user-guide/desktop), [Nix setup](https://hermes-agent.nousresearch.com/docs/getting-started/nix-setup) |
| Claude Code CLI | `claude-code`; run `claude` | [Anthropic setup](https://code.claude.com/docs/en/setup) |
| Codex | `codex`; run `codex` | [OpenAI Codex](https://github.com/openai/codex) |
| Grok CLI | `grok-build`; run `grok`. This selects xAI's official coding agent. Nixpkgs `grok-cli` is a separate community project. | [Grok Build](https://docs.x.ai/build/overview) |
| Cursor Desktop | `code-cursor`; run `cursor` | [Cursor download](https://cursor.com/download) |
| Cursor CLI | `cursor-cli`; run `cursor-agent` | [Cursor CLI](https://cursor.com/docs/cli/overview) |
| Muse Code | Manual setup; no verified Nix package integrated. `pkgs.muse` is a music application. | [Meta Muse Code](https://dev.meta.ai/docs/muse-code) |
| Grokbot | Grok Bot; unavailable for managed Linux installation. The official page lists macOS, Windows and iOS. Do not substitute the unrelated community `grok-bot-cli`. | [Grok Bot](https://x.ai/bot) |
| Mistral Vibe CLI | `mistral-vibe`; run `vibe`. Its packaged `textual-speedups` dependency requires `allowUnfree`. | [Mistral Vibe](https://github.com/mistralai/mistral-vibe) |
| Kimi Code CLI | Manual setup; current MoonshotAI CLI has no verified Nix package integrated here. | [MoonshotAI Kimi Code](https://github.com/MoonshotAI/kimi-code) |
| OpenCode | `opencode`; run `opencode` | [OpenCode](https://github.com/anomalyco/opencode) |
| Pi Coding Agent | `pi-coding-agent`; run `pi` | [Pi](https://pi.dev/) |

Ollama remains available. AI provider authentication and any subscriptions are
configured in each tool after installation. No provider credentials or agents
are enabled automatically. Proprietary package selections add the existing
`allowUnfree` module. A manually injected selection for an unavailable entry
fails with an explicit explanation; it is never silently treated as installed.

## Development Tools

| Requested tool | Nix package | Verified source / details |
| --- | --- | --- |
| Distrobox | `distrobox` | [Distrobox](https://distrobox.it/); enables rootless Podman. |
| DistroShelf | `distroshelf`, `distrobox` | [DistroShelf](https://github.com/ranfdev/DistroShelf); enables rootless Podman even when Distrobox is unchecked. Choose an installed terminal in its preferences. |
| npm | `nodejs` | [npm documentation](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm/); npm ships with Node.js. |
| Bun | `bun` | [Bun](https://bun.sh/docs/installation) |
| Ruby | `ruby` | [Ruby](https://www.ruby-lang.org/en/) |
| Ruby on Rails | `rubyPackages.rails` | [Rails](https://guides.rubyonrails.org/install_ruby_on_rails.html); `rails-new` is a separate project generator. |
| Zig | `zig` | [Zig](https://ziglang.org/download/) |
| Python | `python3` | [Python](https://www.python.org/downloads/); existing entry retained. |
| Flutter | `flutter` | [Flutter](https://docs.flutter.dev/install) |
| Dart | `dart` when selected alone | [Dart](https://dart.dev/get-dart); Flutter already includes its matching Dart SDK. When both are selected, use Flutter's Dart, avoiding duplicate executables. |
| Go | `go` | [Go](https://go.dev/doc/install) |
| Java | `jdk` | [OpenJDK](https://openjdk.org/install/); includes compiler and runtime. |
| JavaScript | `nodejs` | [Node.js](https://nodejs.org/en/about); shares one package with Node.js/npm selections. |
| C++ | `gcc` | [GCC](https://gcc.gnu.org/); C and C++ compiler drivers. |
| GTK4 | `gtk4` | [GTK](https://www.gtk.org/docs/installations/linux/) |
| Qt6 | `qt6.qtbase` | [Qt modules](https://doc.qt.io/qt-6/qtmodules.html); Core, GUI and Widgets. `qt6.full` no longer evaluates on the audited newer Nixpkgs. |
| TypeScript | `typescript` | [TypeScript](https://www.typescriptlang.org/download/); avoid removed `nodePackages.typescript` on newer Nixpkgs. |

GTK/Qt application builds should declare their development dependencies in a
project shell (`nix develop` or `nix-shell -p pkg-config gtk4`, for example).
Installing a system package does not configure every project's compiler flags,
Qt modules, native gems, Android SDK or language dependencies. Container images
are created by the user in Distrobox/DistroShelf, not during system activation.

## Older NixOS channels

The 25.05 package collection lacks Bazaar, DistroShelf, Grok Build, Cursor CLI,
Mistral Vibe and Pi Coding Agent. For those six entries only, generated modules
prefer the host attribute and fall back to a fixed package collection if absent:

- Nixpkgs revision: `c59305bab2065cfecc4944690d9eedbb56f3a9fa`
- Archive: `https://github.com/NixOS/nixpkgs/archive/c59305bab2065cfecc4944690d9eedbb56f3a9fa.tar.gz`
- NAR hash: `sha256-69xHQhAeMAD2wDXO7T2pcOZIF9Sga2W+JkmY2a11Ops=`

This imports packages only, using the host architecture and unfree setting.
It does not update the host's channel, NixOS modules, kernel or state version.
The fetch is lazy and hash checked; it is unnecessary when all selected packages
exist in the host collection. Extra package closures can increase download and
store size. Existing host attributes retain the versions supplied by that host.

## Validation

`scripts/test-generated-nix.py` covers each installable entry in these three
bundles individually, all selected, and empty selections. Rust and Flutter
regressions cover dependency activation, deselection, runtime deduplication,
proprietary packages, setup-only entries and the expanded catalog UI.

Generated package lists carry validated JSON selection markers. The helper uses
them to recover exact checkbox IDs if `state.json` is missing or corrupt, while
retaining the parser for older files. This preserves npm/JavaScript aliases,
Flutter/Dart selections and DistroShelf without inventing a Distrobox selection.

Local checks passed: 157 Rust tests, 33 Flutter tests, workspace Clippy, Flutter
analysis, 26 isolated helper tests, 85 bundle evaluations on the pinned package
collection, and 37 individual plus three whole-bundle evaluations on NixOS 25.05.

Package/module evaluation confirms attribute names and configuration validity;
it does not establish that every provider login or AI model request works.
Those require the user's provider accounts and separate application testing.
