# Ginseng Development Guidelines

Peer-to-peer file sharing app: Rust (Tauri) + React/TypeScript + Iroh networking.

## Architecture
- `src-tauri/src/core.rs` - Pure business logic
- `src-tauri/src/commands.rs` - Tauri command wrappers
- `src-tauri/src/lib.rs` - Tauri app setup and configuration
- `src/components/FileTransfer.tsx` - Main UI component

## Code Standards
**Rust**: Use `anyhow::Result`, never `unwrap()`. All networking is async/await. Must pass `cargo fmt` and `cargo clippy -D warnings`.

**TypeScript**: Functional components with hooks only. Strict compilation required. Always wrap Tauri invokes in try/catch with toast error handling.

**Security**: Validate file paths with `canonicalize()`. Never bypass Iroh encryption. No plaintext transmission.

## Key Dependencies
- `iroh = "0.93.2"` - P2P networking
- `tauri = "2"` - Native framework  
- `react = "^19.1.0"` - Frontend
- `anyhow = "1.0.100"` - Error handling
- `bun` - Package manager and build tool (use instead of npm)

## Requirements
- No servers, accounts, or central coordination
- Direct peer connections only
- End-to-end encryption via Iroh