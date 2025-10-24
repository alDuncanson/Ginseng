> [!WARNING]
> This project is in beta! I just wanted to build in public as I worked through some ideas around peer-to-peer file sharing. Expect bugs, incomplete features, and breaking changes. Use at your own risk.

> [!NOTE]
> This is a passion project in active development. While I'm not accepting feature contributions right now, this is free and open source software—you have every freedom to study it, copy it, modify it, and run it yourself. See the [LICENSE](LICENSE) for details.

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

This approach offers privacy by design—no data passes through external services, and no accounts or profiles are required. Ginseng is developed openly to keep these tools accessible and transparent.

## Quick Start

### With Nix:

Ginseng uses Nix Flakes for reproducible development environments:

```bash
nix develop               # Enter development shell
nix run .#dev             # Launch development build
nix run .#build           # Create release bundles
nix run .#test            # Run test suites
nix run .#format          # Run formatters and linters
```

### Without Nix:
1. Install [Rust](https://www.rust-lang.org/tools/install) and [Bun](https://bun.sh/).

2. Clone the repository:
```bash
git clone https://github.com/alDuncanson/ginseng.git
cd ginseng
```

3. Install dependencies:
```bash
bun install
```

4. Start development build:
```bash
bun x tauri dev
```

## Community

Join the conversation in GitHub Discussions to share ideas, provide feedback, or discuss peer-to-peer technology and digital sovereignty.

## Disclaimer

I, like many others, use AI tools to help write documentation. While I strive to ensure accuracy, please be aware that AI-generated content may contain errors.

If you notice anything wrong, outdated, unclear, or whatever, let me know!

I'm only human.