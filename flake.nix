{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    git-hooks-nix.url = "github:cachix/git-hooks.nix";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = inputs @ {
    flake-parts,
    nixpkgs,
    rust-overlay,
    git-hooks-nix,
    treefmt-nix,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      imports = [
        git-hooks-nix.flakeModule
        treefmt-nix.flakeModule
      ];

      perSystem = {
        self',
        config,
        pkgs,
        system,
        ...
      }: let
        overlays = [(import rust-overlay)];
        pkgsWithOverlay = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgsWithOverlay.rust-bin.nightly.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };
      in {
        packages = {
          default = self'.packages.org-mcp;
          org-mcp = pkgsWithOverlay.symlinkJoin {
            name = "org-mcp";
            paths = [
              self'.packages.org-mcp-server
              self'.packages.org-mcp-cli
            ];
            meta = with pkgs.lib; {
              description = "OrgMode MCP Server Full";
              homepage = "https://github.com/szaffarano/org-mcp-server";
              license = licenses.mit;
              maintainers = [];
            };
          };
          org-mcp-server = pkgsWithOverlay.rustPlatform.buildRustPackage {
            pname = "org-mcp-server";
            version = let
              cargoToml = builtins.fromTOML (builtins.readFile ./mcp-server/Cargo.toml);
            in
              cargoToml.package.version;

            src = pkgs.lib.cleanSource ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = [rustToolchain];

            meta = with pkgs.lib; {
              description = "OrgMode MCP Server";
              homepage = "https://github.com/szaffarano/org-mcp-server";
              license = licenses.mit;
              maintainers = [];
            };
          };
          org-mcp-cli = pkgsWithOverlay.rustPlatform.buildRustPackage {
            pname = "org-mcp-cli";
            version = let
              cargoToml = builtins.fromTOML (builtins.readFile ./org-cli/Cargo.toml);
            in
              cargoToml.package.version;

            src = pkgs.lib.cleanSource ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = [rustToolchain];

            meta = with pkgs.lib; {
              description = "OrgMode MCP CLI";
              homepage = "https://github.com/szaffarano/org-mcp-cli";
              license = licenses.mit;
              maintainers = [];
            };
          };
        };
        pre-commit = {
          check.enable = true;
          settings = {
            hooks = {
              alejandra.enable = true;
              rustfmt.enable = true;
              deadnix.enable = true;
              statix.enable = true;
            };
          };
        };
        treefmt = {
          projectRootFile = "flake.nix";
          programs = {
            alejandra.enable = true;
            deadnix.enable = true;
            statix.enable = true;
            jsonfmt.enable = true;
            mdformat.enable = true;
            shfmt.enable = true;
            toml-sort.enable = true;
            yamlfmt.enable = true;
          };
          settings = {
            on-unmatched = "info";
            excludes = [
              "Cargo.toml"
              "*.conf"
              "*.css"
              "flake.lock"
              "*.ini"
              "*.pub"
            ];
          };
          settings.formatter.shfmt = {
            includes = [
              "*.sh"
            ];
          };
        };

        devShells = {
          default = pkgsWithOverlay.mkShell {
            shellHook = ''
              ${config.pre-commit.installationScript}
            '';
            inputsFrom = [self'.packages.org-mcp-server];
            inherit (self'.checks) pre-commit;
            packages = with pkgsWithOverlay; [
              rustToolchain
              cargo-bloat
              cargo-edit
              cargo-outdated
              cargo-udeps
              cargo-watch
              pkgs.alejandra
              pkgs.deadnix
              pkgs.statix
              curl
              git
              jq
            ];

            env = {
              RUST_BACKTRACE = "1";
            };
          };
        };
      };
    };
}
