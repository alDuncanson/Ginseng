# AGENTS.md

Ginseng exemplifies peer-to-peer sovereignty through cryptographic networking—eliminate intermediaries, restore user agency. Built with Rust, Tauri, React, and Iroh's P2P foundation.

## Development Environment

**Nix Flake (Preferred)**:
- `nix develop` - Enter reproducible development shell with Rust toolchain, Bun, and system dependencies
- `nix run .#dev` - Launch Tauri development server
- `nix run .#build` - Generate release bundles for all platforms
- `nix run .#test` - Execute Rust test suite

**Manual Setup**:
- `bun install --frozen-lockfile` - Install frontend dependencies (use exact lockfile)
- `cargo tauri dev` - Start development server with hot reload
- `cargo tauri build` - Create production bundles

**Environment Activation**:
- `direnv allow` - Automatic environment activation (if direnv installed)

## Project Architecture

**Frontend Stack**:
- React 19 + TypeScript (strict mode)
- Vite bundler + hot module replacement
- Tauri 2.0 native integration

**Backend Stack**:
- Rust with async/await patterns (Tokio runtime)
- Iroh peer-to-peer networking library
- Tauri plugins for system integration

**Build System**:
- Nix Flakes for reproducible environments
- Bun for JavaScript tooling and package management
- Cargo for Rust compilation and testing

## Code Conventions

**Rust Standards**:
- Follow `cargo fmt` formatting (enforced in CI)
- Address all `cargo clippy` warnings with `-D warnings` flag
- Implement comprehensive error handling via `anyhow` crate
- Use async patterns consistently throughout networking code

**TypeScript Standards**:
- Strict mode compilation with `--noEmit` verification
- Functional patterns preferred over class-based approaches
- React hooks for state management and side effects

**File Organization**:
- `src-tauri/src/` - Rust backend implementation
- `src/` - React frontend components and logic
- `src-tauri/Cargo.toml` - Rust dependencies and metadata
- `package.json` - Frontend dependencies managed by Bun

## Testing Strategy

**Continuous Integration Validation**:
1. TypeScript compilation: `bun x tsc --noEmit`
2. Rust formatting: `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`
3. Rust linting: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`
4. Rust tests: `cargo test --manifest-path src-tauri/Cargo.toml --all --verbose`

**Pre-commit Requirements**:
- All TypeScript must compile without errors
- Rust code must pass formatting and linting checks
- Unit tests must achieve comprehensive coverage of P2P networking logic
- Integration tests should validate file transfer scenarios

## Security Considerations

**Cryptographic Standards**:
- All peer connections utilize end-to-end encryption via Iroh's QUIC implementation
- Node authentication through ed25519 public key verification
- No plaintext data transmission between peers

**Attack Surface Minimization**:
- No central servers or intermediary dependencies
- Local-first architecture with encrypted storage
- Minimal system permissions through Tauri's security model

## Release Process

**Automated Deployment**:
- Triggered by semantic version tags (`v*.*.*` pattern)
- Multi-platform bundles: macOS (.dmg), Linux (.AppImage), Windows (.msi)
- GitHub Actions orchestrates cross-compilation and artifact distribution

**Version Management**:
- Follow semantic versioning for public API changes
- Update `package.json` and `src-tauri/Cargo.toml` versions synchronously
- Tag releases only after comprehensive testing validation

## Dependencies and Updates

**Critical Libraries**:
- `iroh = "0.93.2"` - Core P2P networking (monitor for security updates)
- `tauri = "2"` - Native application framework
- `tokio = "1.48.0"` - Async runtime for Rust backend
- `react = "^19.1.0"` - Frontend framework

**Update Protocol**:
- Test Iroh updates thoroughly due to networking implications
- Validate Tauri compatibility with system integrations
- Ensure React updates maintain TypeScript compatibility

## Debugging and Diagnostics

**Development Tools**:
- `RUST_BACKTRACE=1` enabled in development environment
- Tauri's developer tools accessible via right-click context menu
- Network debugging through Iroh's logging infrastructure

**Performance Monitoring**:
- Profile P2P connection establishment latency
- Monitor memory usage during large file transfers
- Validate encryption overhead impacts on throughput

## Philosophical Alignment

Maintain unwavering commitment to user sovereignty throughout development:
- Reject features requiring central coordination or data collection
- Prioritize cryptographic verification over convenience compromises  
- Design for resilience against network partitions and adversarial conditions
- Preserve user agency through transparent, auditable implementations