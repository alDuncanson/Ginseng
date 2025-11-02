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

_Free and direct file sharing, globally_

</div>

## What is Ginseng?

Share files directly from your device—for free—with anyone, anywhere on the
planet.

## Roadmap

A few things I'm thinking about (this list will probably change quite a bit as I
figure things out):

- [ ] parallel file processing
- [ ] web support
- [ ] mobile support
- [ ] some sort of contact system so you don't have to copy paste tickets
- [ ] QR codes for ticket sharing

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to
contribute to Ginseng.

## Technologies

Ginseng is a cross-platform desktop application built with
[Tauri](https://v2.tauri.app/). The core is built with
[Rust](https://rust-lang.org/), and uses [Iroh](https://www.iroh.computer/) for
peer-to-peer networking. The interface is constructed with
[React](https://react.dev/) and [shadcn](https://ui.shadcn.com/), styled with
[Tailwind](https://tailwindcss.com/), formatted and linted with
[Biome](https://biomejs.dev/), and built with [Vite](https://vite.dev/).
Dependencies are managed with [Bun](https://bun.sh/) and
[Cargo](https://doc.rust-lang.org/stable/cargo/), and development environments
are reproducible with [Nix](https://nixos.org/).

