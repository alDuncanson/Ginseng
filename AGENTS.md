# Ginseng - Agent Guidelines

## Build & Development
- **Dev**: `bun x tauri dev` (or `nix run .#dev`)
- **Build**: `tsc && vite build` + `bun x tauri build` (or `nix run .#build`)
- **Typecheck**: `tsc --noEmit`
- **Lint**: `biome check .` (fix: `biome check --write .`)
- **Format**: `biome format .` (fix: `biome format --write .`)
- **Rust check**: `cargo check` (in `src-tauri/`)
- **Rust build**: `cargo build` (in `src-tauri/`)

## Architecture
- **Stack**: Tauri v2 app with Rust backend + React frontend
- **Frontend**: React 19 + shadcn/ui + Tailwind CSS + Vite
- **Backend**: Rust (src-tauri/) using Iroh for P2P file sharing, Tauri for desktop APIs
- **Key modules**: `src-tauri/src/core.rs` (Iroh P2P logic), `src-tauri/src/commands.rs` (Tauri commands), `src/components/FileTransfer.tsx` (main UI)
- **Package managers**: Bun for frontend, Cargo for Rust

## Code Style
- **TypeScript**: Strict mode, use type imports (`import type`), tabs (width 2), semicolons, double quotes, 100 char line width, `@/` path alias for `src/`
- **Rust**: Standard Rust formatting, comprehensive doc comments for public items (see `core.rs` examples)
- **Formatting**: Biome enforces all style rules (see biome.json) - tabs, double quotes, semicolons, type imports
- **Linting**: No unused variables (error), exhaustive deps (warn), no explicit any (warn), use import type (error)
- **Naming**: PascalCase for components, camelCase for functions/variables, snake_case for Rust
