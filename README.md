![Demo](./public/demo.gif)

<div align="center">

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

## How It Works

Ginseng uses peer-to-peer networking to transfer files directly between devices
without intermediaries. Here's what makes it special:

**No servers, no storage, no costs**  
Files transfer directly from your device to the recipient's device. Nothing is
stored in the cloud, so there are no server costs to pass on to you.

**End-to-end encrypted**  
Built on [Iroh](https://www.iroh.computer/), Ginseng uses cryptographic hashing
and content addressing to ensure your files are transferred securely. Only
devices with the share ticket can access the files.

**Global connectivity**  
Using hole-punching and relay servers for NAT traversal, Ginseng can connect
peers anywhere in the world—even behind firewalls and routers.

**How sharing works:**

1. Select files or folders to share
2. Ginseng creates a unique, encrypted ticket
3. Share the ticket with your recipient
4. They paste the ticket in their Ginseng app
5. Files transfer directly peer-to-peer

The ticket contains cryptographic information needed to find your device on the
network and verify file integrity, but the actual file content never touches any
server.

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

