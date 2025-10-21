{
  description = "Ginseng's development nix flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true;
          };
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "cargo" ];
        };

        commonInputs = with pkgs; [
          bun
          nixfmt-tree
        ];

        mkScript = name: text: toString (pkgs.writeShellScript name text);
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = commonInputs ++ [ rustToolchain ];

          RUST_BACKTRACE = "1";

          shellHook = ''
            echo ""
            echo "🌿 Welcome to the Ginseng dev shell"
            echo ""
            echo "Development commands:"
            echo "  nix run .#dev               # run Tauri dev mode"
            echo "  nix run .#build             # build Tauri release bundles"
            echo "  nix run .#test              # run Rust tests"
            echo "  nix run .#format .          # format Nix files"
            echo ""
          '';
        };

        apps = {
          dev = {
            type = "app";
            program = mkScript "ginseng-dev" ''
              exec ${rustToolchain}/bin/cargo tauri dev "$@"
            '';
            meta.description = "Run Tauri in development mode";
          };

          build = {
            type = "app";
            program = mkScript "ginseng-build" ''
              exec ${rustToolchain}/bin/cargo tauri build "$@"
            '';
            meta.description = "Build Tauri release bundles";
          };

          test = {
            type = "app";
            program = mkScript "ginseng-test" ''
              exec ${rustToolchain}/bin/cargo test --manifest-path src-tauri/Cargo.toml "$@"
            '';
            meta.description = "Run Rust tests via cargo";
          };

          format = {
            type = "app";
            program = mkScript "ginseng-format" ''
              exec ${pkgs.nixfmt-tree}/bin/treefmt "$@"
            '';
            meta.description = "Format Nix files with nixfmt-tree";
          };
        };

        formatter = pkgs.nixfmt-tree;
      }
    );
}
