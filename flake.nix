{
  description = "NixOS Toolkit - A GTK4/libadwaita GUI for declarative NixOS management";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    crane = {
      url = "github:ipetkov/crane";
    };

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, advisory-db }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        # Rust toolchain - stable with rust-src for IDE support
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Source filtering - include Rust files and templates
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (builtins.match ".*\\.nix$" path != null)
            || (builtins.match ".*/nix/templates/.*" path != null)
            || (builtins.match ".*/data/.*" path != null);
        };

        # Common build arguments
        commonArgs = {
          inherit src;
          strictDeps = true;
          pname = "nixos-toolkit";
          version = "0.1.0";

          nativeBuildInputs = with pkgs; [
            pkg-config
            wrapGAppsHook4
            glib
          ];

          buildInputs = with pkgs; [
            # GTK4 and libadwaita
            gtk4
            libadwaita
            glib
            gdk-pixbuf
            graphene
            pango
            cairo

            # GSettings
            gsettings-desktop-schemas
            dconf

            # System
            openssl

            # Clipboard
            wl-clipboard
            xclip
          ];
        };

        # Build dependencies only (for caching)
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Template files derivation
        templateFiles = pkgs.runCommand "nixos-toolkit-templates" { } ''
          mkdir -p $out/share/nixos-toolkit/templates/{profiles,bundles,state}
          cp -r ${./nix/templates/profiles}/* $out/share/nixos-toolkit/templates/profiles/ 2>/dev/null || true
          cp -r ${./nix/templates/bundles}/* $out/share/nixos-toolkit/templates/bundles/ 2>/dev/null || true
          cp -r ${./nix/templates/state}/* $out/share/nixos-toolkit/templates/state/ 2>/dev/null || true
        '';

        # GUI application
        nixos-toolkit-gui = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "nixos-toolkit";
          cargoExtraArgs = "-p gui";

          postInstall = ''
            # Rename binary
            mv $out/bin/gui $out/bin/nixos-toolkit

            # Install desktop file
            install -Dm644 ${./data/nixos-toolkit.desktop} $out/share/applications/nixos-toolkit.desktop || true

            # Install icon
            install -Dm644 ${./data/icons/nixos-toolkit.svg} $out/share/icons/hicolor/scalable/apps/nixos-toolkit.svg || true

            # Link templates
            ln -sf ${templateFiles}/share/nixos-toolkit $out/share/nixos-toolkit
          '';

          preFixup = ''
            gappsWrapperArgs+=(
              --set NIXOS_TOOLKIT_TEMPLATES_DIR "$out/share/nixos-toolkit/templates"
              --set NIXOS_TOOLKIT_HELPER "${nixos-toolkit-helper}/bin/nixos-toolkit-helper"
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.wl-clipboard pkgs.xclip ]}
            )
          '';

          meta = with pkgs.lib; {
            description = "NixOS Toolkit - Declarative system management GUI";
            homepage = "https://github.com/devjonesafrica/NixOSApp";
            license = licenses.gpl3Plus;
            mainProgram = "nixos-toolkit";
            platforms = platforms.linux;
          };
        });

        # Helper binary (privileged operations)
        nixos-toolkit-helper = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "nixos-toolkit-helper";
          cargoExtraArgs = "-p helper";

          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [ pkgs.makeWrapper ];

          postInstall = ''
            mv $out/bin/helper $out/bin/nixos-toolkit-helper

            # Install polkit policy
            install -Dm644 ${./data/polkit/org.nixos-toolkit.helper.policy} \
              $out/share/polkit-1/actions/org.nixos-toolkit.helper.policy || true
          '';

          preFixup = ''
            wrapProgram $out/bin/nixos-toolkit-helper \
              --set NIXOS_TOOLKIT_TEMPLATES_DIR "${templateFiles}/share/nixos-toolkit/templates" \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nix pkgs.nixos-rebuild pkgs.git pkgs.hostname ]}
          '';

          meta = with pkgs.lib; {
            description = "NixOS Toolkit Helper - Privileged operations";
            license = licenses.gpl3Plus;
            mainProgram = "nixos-toolkit-helper";
            platforms = platforms.linux;
          };
        });

      in
      {
        # Checks
        checks = {
          inherit nixos-toolkit-gui nixos-toolkit-helper;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          fmt = craneLib.cargoFmt { inherit src; };

          audit = craneLib.cargoAudit { inherit src advisory-db; };
        };

        # Packages
        packages = {
          default = nixos-toolkit-gui;
          gui = nixos-toolkit-gui;
          helper = nixos-toolkit-helper;
          templates = templateFiles;

          full = pkgs.symlinkJoin {
            name = "nixos-toolkit-full";
            paths = [ nixos-toolkit-gui nixos-toolkit-helper templateFiles ];
          };
        };

        # Apps for `nix run`
        apps = {
          default = flake-utils.lib.mkApp {
            drv = nixos-toolkit-gui;
            name = "nixos-toolkit";
          };
          helper = flake-utils.lib.mkApp {
            drv = nixos-toolkit-helper;
            name = "nixos-toolkit-helper";
          };
        };

        # Development shell
        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = with pkgs; [
            # Rust tools
            rust-analyzer
            cargo-watch
            cargo-edit
            cargo-expand

            # GTK development
            gtk4.dev
            libadwaita.dev
            gobject-introspection

            # Nix tools
            nil
            nixpkgs-fmt

            # Debug
            gdb
          ];

          shellHook = ''
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
            export GSETTINGS_SCHEMA_DIR="${pkgs.glib.getSchemaPath pkgs.gtk4}"
            export GIO_EXTRA_MODULES="${pkgs.dconf.lib}/lib/gio/modules"
            export NIXOS_TOOLKIT_TEMPLATES_DIR="$PWD/nix/templates"
            echo "NixOS Toolkit Development Environment"
            echo "Run: cargo run -p gui"
          '';
        };
      }
    ) // {
      # NixOS module for system-wide installation
      nixosModules.default = { config, lib, pkgs, ... }:
        let
          cfg = config.programs.nixos-toolkit;
        in
        {
          options.programs.nixos-toolkit = {
            enable = lib.mkEnableOption "NixOS Toolkit GUI management application";

            package = lib.mkOption {
              type = lib.types.package;
              default = self.packages.${pkgs.system}.default;
              description = "The nixos-toolkit package to use";
            };
          };

          config = lib.mkIf cfg.enable {
            environment.systemPackages = [
              cfg.package
              self.packages.${pkgs.system}.helper
            ];
          };
        };

      # Overlay
      overlays.default = final: prev: {
        nixos-toolkit = self.packages.${final.system}.default;
        nixos-toolkit-helper = self.packages.${final.system}.helper;
      };
    };
}
