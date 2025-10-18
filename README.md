# Ginseng

> [!NOTE] 
> coming soon

Ginseng is a peer-to-peer file sharing application that connects users directly without requiring central servers. Share files between devices while keeping your data private and under your control. Built to be simple and accessible, with no fees or account registration needed.

## nix flake

This repo includes a Nix flake for a ready-to-use development shell inspired by my soil repo.

- Enter the dev shell: `nix develop`
- Run Tauri dev: `nix run .#dev`
- Build release bundles: `nix run .#build`

Inside the shell, you’ll have Rust (with rustfmt and clippy) and Bun available. Common tasks:

- bun install
- bun run dev
- bun run tauri dev
- bun run tauri build

If you use direnv, `.envrc` is provided; allow it with `direnv allow`.

## ci/cd

- CI (`.github/workflows/ci.yml`) runs on pushes and PRs to main:
	- Typechecks the frontend via TypeScript
	- Runs rustfmt, clippy, and tests for the `src-tauri` crate

- Releases (`.github/workflows/release.yml`) are triggered by tags matching `v*.*.*` and use the official Tauri action to build and upload native bundles for macOS, Linux, and Windows.

### release prerequisites

- No special secrets are required beyond the default `GITHUB_TOKEN`
- Ensure the repo has a tag like `v0.1.0` to trigger a release build