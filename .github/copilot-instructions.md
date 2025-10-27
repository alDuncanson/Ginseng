# Ginseng Development Guidelines

Peer-to-peer file sharing app: Rust (Tauri) + React/TypeScript + Iroh networking.

## Architecture
### Backend (Rust/Tauri)
- `src-tauri/src/core.rs` - Pure business logic (networking, Iroh protocols, share/download APIs)
- `src-tauri/src/state.rs` - Application state management (AppState, setup functions)
- `src-tauri/src/commands.rs` - Tauri command interface (pure command handlers)
- `src-tauri/src/utils.rs` - File utilities (path validation, canonicalization, downloads directory)
- `src-tauri/src/lib.rs` - Tauri app setup and module orchestration

### Frontend (React/TypeScript)
- `src/components/FileTransfer.tsx` - Main UI component

## Code Standards
**Rust**: Use `anyhow::Result`, never `unwrap()`. All networking is async/await. Must pass `cargo fmt` and `cargo clippy -D warnings`.

**TypeScript**: Functional components with hooks only. Strict compilation required. Always wrap Tauri invokes in try/catch with toast error handling. Use proper keys in React lists (avoid array indices).

**Security**: Validate file paths with `canonicalize()`. Never bypass Iroh encryption. No plaintext transmission.

## Build & Verification
**DO NOT run `cargo build`** - Tauri app signing fails locally and only works in CI.

**Instead, verify your work with:**
- `cargo check` - Fast compilation check
- `cargo test` - Run all tests
- `cargo clippy` - Lint and catch issues
- `cargo fmt` - Code formatting
- `bun run check:fix` - Frontend formatting and linting with Biome

## Key Dependencies
- `iroh = "0.94.0"` - P2P networking
- `tauri = "2"` - Native framework  
- `react = "^19.1.0"` - Frontend
- `anyhow = "1.0.100"` - Error handling
- `bun` - Package manager and build tool (use instead of npm)

## UI Components
**shadcn/ui**: Use shadcn for new UI components. Add with `bunx --bun shadcn@latest add <component>` (e.g., `bunx --bun shadcn@latest add accordion`). Components are in `src/components/ui/`.

## Module Responsibilities
**core.rs**: Contains all Iroh networking logic, blob storage, and P2P operations. Never import from other modules except utils for pure utilities.

**state.rs**: Manages Tauri application state lifecycle. Only imports from core.rs.

**commands.rs**: Thin wrapper layer for Tauri commands. Imports from state.rs and utils.rs. No business logic.

**utils.rs**: Pure utility functions with no external dependencies except standard library and basic crates (anyhow, dirs). Can be used by any module.

## Requirements
- No servers, accounts, or central coordination
- Direct peer connections only
- End-to-end encryption via Iroh