
# Contributing to Ginseng

Thank you for your interest in Ginseng! This project is maintained in my free time and contributions are appreciated, but please read the following before opening an issue or pull request.

If you have a question or a feature request, [start a discussion](https://github.com/alDuncanson/Ginseng/discussions).

If you find a bug, please open an [issue](https://github.com/alDuncanson/Ginseng/issues).

I can't guarantee that I'll respond to every issue or pull request, or that contributions will be merged. I'm still exploring the design and direction of this project, and may choose to implement features differently or not at all.

## Run the Development Environment

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