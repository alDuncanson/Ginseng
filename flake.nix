{
  description = "Ginseng - Tauri app dev shell and helper apps";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfree = true;
          };
        };

        # Rust toolchain with common components
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rustfmt" "clippy" ];
        };

        # Common libs and build tools (minimal)
        commonInputs = with pkgs; [
          pkg-config
          openssl
          # Frontend tooling required for Tauri/Vite steps
          bun
        ];

        # Simple helper to make small wrapper apps
        mkScript = name: text: toString (pkgs.writeShellScript name text);
      in
      {
        # Dev shell with minimal tools: Rust toolchain + Bun
        devShells.default = pkgs.mkShell {
          buildInputs = commonInputs ++ [ rustToolchain ];

          # Useful env for Rust backtraces
          RUST_BACKTRACE = "1";

          shellHook = ''
            echo "🌿 Welcome to the Ginseng dev shell"
            echo ""
            echo "Common tasks (cargo-first):"
            echo "  cargo build --manifest-path src-tauri/Cargo.toml    # build Rust backend"
            echo "  cargo test  --manifest-path src-tauri/Cargo.toml    # run Rust tests"
            echo "  cargo clippy --manifest-path src-tauri/Cargo.toml   # lint Rust"
            echo ""
            echo "Tauri (requires Bun for frontend tooling):"
            echo "  bun install                 # install JS deps"
            echo "  bun run tauri dev           # run Tauri dev"
            echo "  bun run tauri build         # build Tauri bundles"
          '';
        };

        # Apps for convenience: nix run .#dev / .#build
        apps = {
          dev = {
            type = "app";
            program = mkScript "ginseng-dev" ''
              exec ${pkgs.bun}/bin/bun run tauri dev "$@"
            '';
            meta.description = "Run Tauri in development mode";
          };

          build = {
            type = "app";
            program = mkScript "ginseng-build" ''
              exec ${pkgs.bun}/bin/bun run tauri build "$@"
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
        };

        # Formatter for `nix fmt`
        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
