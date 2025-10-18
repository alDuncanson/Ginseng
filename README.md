> [!WARNING]
> This project is in early development. Features, stability, and security guarantees are not yet established. Use at your own risk.

<div align="center">

<img src="src-tauri/icons/128x128.png" alt="Ginseng" width="80" height="80">

# Ginseng

[![CI](https://github.com/alDuncanson/ginseng/actions/workflows/ci.yml/badge.svg)](https://github.com/alDuncanson/ginseng/actions/workflows/ci.yml)
[![Release](https://github.com/alDuncanson/ginseng/actions/workflows/release.yml/badge.svg)](https://github.com/alDuncanson/ginseng/releases)
[![GitHub Stars](https://img.shields.io/github/stars/alDuncanson/ginseng?style=flat-square)](https://github.com/alDuncanson/ginseng/stargazers)
[![GitHub Release](https://img.shields.io/github/v/release/alDuncanson/ginseng?style=flat-square)](https://github.com/alDuncanson/ginseng/releases)

[![Nix Flakes](https://img.shields.io/badge/Nix-Flakes-5277C3?style=flat-square&logo=nixos)](https://nixos.org)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB?style=flat-square&logo=tauri)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.80+-000000?style=flat-square&logo=rust)](https://www.rust-lang.org)
[![React](https://img.shields.io/badge/React-19.1-61DAFB?style=flat-square&logo=react)](https://react.dev)
[![Bun](https://img.shields.io/badge/Bun-1.3+-000000?style=flat-square&logo=bun)](https://bun.sh)
[![Iroh](https://img.shields.io/badge/Iroh-0.93-FF6B6B?style=flat-square)](https://iroh.computer)

*Native peer-to-peer file sharing that restores user agency*

</div>

## Philosophy

Ginseng embodies the conviction that users deserve sovereignty over their data and connections. Built atop [Iroh's](https://iroh.computer) peer-to-peer foundation, it eliminates intermediaries, circumvents surveillance capitalism, and returns computational autonomy to individuals. Your devices communicate directly—encrypted, authenticated, unmediated—transforming powerful hardware into genuine peers rather than subordinate clients.

No accounts. No servers. No gatekeepers. Just cryptographically secure connections between devices you control.

## Quick Start

**Nix Flake** (recommended):
```bash
nix develop                # Enter development shell
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