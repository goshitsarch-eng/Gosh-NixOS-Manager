{
  description = "NixOS Toolkit - A libcosmic GUI for declarative NixOS management";

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

        # Rust toolchain - stable with rust-src for IDE support (rust-version 1.93+)
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Source filtering - include Rust files, templates, i18n, and data
        src = pkgs.lib.cleanSourceWith {
          src = ./.;
          filter = path: type:
            (craneLib.filterCargoSources path type)
            || (builtins.match ".*\\.nix$" path != null)
            || (builtins.match ".*/nix/templates/.*" path != null)
            || (builtins.match ".*/data/.*" path != null)
            || (builtins.match ".*\\.ftl$" path != null)
            || (builtins.match ".*/i18n/.*" path != null)
            || (builtins.match ".*i18n.toml$" path != null);
        };

        guiNativeInputs = with pkgs; [
          pkg-config
          makeWrapper
        ];

        guiBuildInputs = with pkgs; [
          openssl
          libxkbcommon
          wayland
          expat
          fontconfig
          freetype
          mesa
          libGL
          xorg.libX11
          xorg.libXcursor
          xorg.libXrandr
          xorg.libXi
          xorg.libXext
        ];

        helperNativeInputs = with pkgs; [
          pkg-config
          makeWrapper
        ];

        helperBuildInputs = with pkgs; [
          openssl
        ];

        guiArgs = {
          inherit src;
          strictDeps = true;
          pname = "nixos-toolkit";
          version = "0.1.0";
          nativeBuildInputs = guiNativeInputs;
          buildInputs = guiBuildInputs;
        };

        helperArgs = {
          inherit src;
          strictDeps = true;
          pname = "nixos-toolkit-helper";
          version = "0.1.0";
          nativeBuildInputs = helperNativeInputs;
          buildInputs = helperBuildInputs;
        };

        guiCargoArtifacts = craneLib.buildDepsOnly (guiArgs // {
          cargoExtraArgs = "-p gui";
        });

        helperCargoArtifacts = craneLib.buildDepsOnly (helperArgs // {
          cargoExtraArgs = "-p helper";
        });

        # Template files derivation
        templateFiles = pkgs.runCommand "nixos-toolkit-templates" { } ''
          mkdir -p $out/share/nixos-toolkit/templates/{profiles,bundles,state}
          cp -r ${./nix/templates/profiles}/* $out/share/nixos-toolkit/templates/profiles/ 2>/dev/null || true
          cp -r ${./nix/templates/bundles}/* $out/share/nixos-toolkit/templates/bundles/ 2>/dev/null || true
          cp -r ${./nix/templates/state}/* $out/share/nixos-toolkit/templates/state/ 2>/dev/null || true
        '';

        # GUI application (libcosmic; no GTK / wrapGAppsHook4)
        nixos-toolkit-gui = craneLib.buildPackage (guiArgs // {
          cargoArtifacts = guiCargoArtifacts;
          pname = "nixos-toolkit";
          cargoExtraArgs = "-p gui";

          postInstall = ''
            # Install desktop file
            install -Dm644 ${./data/nixos-toolkit.desktop} $out/share/applications/nixos-toolkit.desktop || true

            # Install icon
            install -Dm644 ${./data/icons/nixos-toolkit.svg} $out/share/icons/hicolor/scalable/apps/nixos-toolkit.svg || true

            # Link templates
            ln -sf ${templateFiles}/share/nixos-toolkit $out/share/nixos-toolkit
          '';

          preFixup = ''
            wrapProgram $out/bin/nixos-toolkit \
              --set NIXOS_TOOLKIT_TEMPLATES_DIR "$out/share/nixos-toolkit/templates" \
              --set NIXOS_TOOLKIT_HELPER "${nixos-toolkit-helper}/bin/nixos-toolkit-helper"
          '';

          meta = with pkgs.lib; {
            description = "NixOS Toolkit - Declarative system management GUI";
            homepage = "https://github.com/goshitsarch-eng/Gosh-NixOS-Manager";
            license = licenses.gpl3Plus;
            mainProgram = "nixos-toolkit";
            platforms = platforms.linux;
          };
        });

        # Helper binary (privileged operations)
        nixos-toolkit-helper = craneLib.buildPackage (helperArgs // {
          cargoArtifacts = helperCargoArtifacts;
          pname = "nixos-toolkit-helper";
          cargoExtraArgs = "-p helper";

          postInstall = ''
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

          clippy = craneLib.cargoClippy (guiArgs // {
            cargoArtifacts = guiCargoArtifacts;
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
            rust-analyzer
            cargo-watch
            cargo-edit
            cargo-expand
            pkg-config
            libxkbcommon
            wayland
            expat
            fontconfig
            freetype
            mesa
            nil
            nixpkgs-fmt
            gdb
          ];

          shellHook = ''
            export RUST_SRC_PATH="${rustToolchain}/lib/rustlib/src/rust/library"
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
