> [!WARNING]
> This project is in early development. Features, stability, and security guarantees are not yet established. Use at your own risk.

> [!NOTE]
> This is a passion project in active development. While I'm not accepting feature contributions right now as I work through the project's direction and core vision, this is free and open source software—you have every freedom to study it, copy it, run it yourself, and modify it as you see fit. See [CONTRIBUTING.md](CONTRIBUTING.md) for details on how you can help during this phase.

<div align="center">

<img src="src-tauri/icons/128x128.png" alt="Ginseng" width="80" height="80">

# Ginseng

[![CI](https://github.com/alDuncanson/ginseng/actions/workflows/ci.yml/badge.svg)](https://github.com/alDuncanson/ginseng/actions/workflows/ci.yml)
[![Release](https://github.com/alDuncanson/ginseng/actions/workflows/release.yml/badge.svg)](https://github.com/alDuncanson/ginseng/releases)
[![CodeQL](https://github.com/alDuncanson/Ginseng/actions/workflows/github-code-scanning/codeql/badge.svg)](https://github.com/alDuncanson/Ginseng/actions/workflows/github-code-scanning/codeql)
[![Deploy site](https://github.com/alDuncanson/Ginseng/actions/workflows/static.yml/badge.svg)](https://github.com/alDuncanson/Ginseng/actions/workflows/static.yml)

[![GitHub Downloads](https://img.shields.io/github/downloads/alDuncanson/ginseng/total?style=flat-square)](https://github.com/alDuncanson/ginseng/releases)
[![GitHub Stars](https://img.shields.io/github/stars/alDuncanson/ginseng?style=flat-square)](https://github.com/alDuncanson/ginseng/stargazers)
[![GitHub Release](https://img.shields.io/github/v/release/alDuncanson/ginseng?include_prereleases&style=flat-square)](https://github.com/alDuncanson/ginseng/releases)

*Native peer-to-peer file sharing that restores user agency*

</div>

## Philosophy

Ginseng is built on the idea that file sharing should be simple and direct. Using [Iroh's](https://iroh.computer) peer-to-peer networking, your devices connect to each other without relying on third-party servers or accounts. Files are encrypted and authenticated, traveling directly between devices.

This approach offers privacy by design—no data passes through external services, and no accounts or profiles are required. The project is developed openly to keep these tools accessible and transparent.

## Quick Start

**Nix Flake** (recommended):
```bash
nix develop               # Enter development shell
nix run .#dev             # Launch development build
nix run .#build           # Create release bundles
```

**Manual Setup**:
```bash
bun install               # Install dependencies  
bun run tauri dev         # Development server
bun run tauri build       # Production build
```

With direnv: `direnv allow` activates the development environment automatically.

## Architecture

- **Frontend**: React 19 + TypeScript + Vite
- **Backend**: Rust + Tauri 2.0 + Iroh networking
- **Build System**: Nix Flakes + Bun + Cargo
- **Platforms**: macOS, Linux, Windows (native bundles)

## Development

CI validates TypeScript compilation, Rust formatting, linting, and test suites. Releases trigger automatically on `v*.*.*` tags, generating platform-specific binaries via GitHub Actions.

The Nix flake provides reproducible development environments with Rust toolchain, Bun runtime, and all system dependencies pre-configured.

## Community

Join the conversation in [GitHub Discussions](https://github.com/alDuncanson/ginseng/discussions) to share ideas, provide feedback, or discuss peer-to-peer technology and digital sovereignty.