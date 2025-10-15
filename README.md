# 🔒 jail-ai

A Rust-based jail wrapper for sandboxing AI agents using podman. Provides isolation, resource limits, and workspace management for secure AI agent execution.

[![Crates.io](https://img.shields.io/crates/v/jail-ai.svg)](https://crates.io/crates/jail-ai)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## ✨ Features

- 🤖 **AI Agent Integration**: Pre-configured support for Claude Code, GitHub Copilot CLI, Cursor Agent, Gemini CLI, and Codex CLI
- 🏗️ **Layered Container System**: Smart image building with automatic project type detection (Rust, Go, Node.js, Python, Java, PHP, C/C++, C#, Terraform, Kubernetes)
- 📦 **Nix Flakes Support**: Automatic detection and loading of Nix development environments
- 🔄 **Workspace Auto-mounting**: Current directory automatically mounted to `/workspace` in the jail
- 🔒 **Minimal Auth by Default**: Claude auto-mounts only API credentials; other agents require explicit flags
- 🌍 **Environment Inheritance**: Automatically inherits `TERM`, timezone, and SSH agent socket
- 🔐 **Opt-in Git/GPG**: Enable git configuration and GPG signing with `--git-gpg` flag
- 🛡️ **Resource Limits**: Memory and CPU quota restrictions
- 🌐 **Network Isolation**: Configurable network access (disabled, private, or shared)
- 📁 **Bind Mounts**: Support for read-only and read-write mounts

## 🚀 Installation

### From Source

```bash
git clone https://github.com/cyrinux/jail-ai.git
cd jail-ai
cargo build --release
sudo cp target/release/jail-ai /usr/local/bin/
```

### From Crates.io

```bash
cargo install jail-ai
```

## 📋 Prerequisites

- **podman** - Container runtime
- **Rust toolchain** (for building from source)

## 🎯 Quick Start

```bash
# Create a jail with auto-mounted workspace (auto-builds image if needed)
jail-ai create my-agent

# Execute a command in the jail
jail-ai exec my-agent -- ls -la /workspace

# Start an interactive shell
jail-ai exec my-agent --interactive -- bash

# Claude Code with minimal auth (auto-mounts credentials only)
jail-ai claude -- chat "help me debug this code"

# GitHub Copilot with full config
jail-ai copilot --copilot-dir -- suggest "write tests"

# Cursor Agent with full config
jail-ai cursor --cursor-dir -- analyze

# Gemini CLI with full config
jail-ai gemini --gemini-dir -- --model gemini-pro "explain this"

# Codex CLI - Open interactive shell for OAuth authentication
jail-ai codex --codex-dir --auth

# Codex CLI - Run agent after authentication is complete
jail-ai codex --codex-dir -- generate "create a REST API"

# Start interactive shell in Claude jail (without running Claude)
jail-ai claude --shell
```

## 🏗️ Layered Image System

jail-ai uses a smart layered image system that automatically detects your project type and builds appropriate container images:

### Image Layers

1. **Base Layer** (`localhost/jail-ai-base:latest`)
   - Shell: zsh with Powerlevel10k theme, bash
   - Shell enhancements: fzf, ripgrep, fd-find
   - Utilities: git, vim, nano, helix, jq, tree, tmux, htop, gh CLI

2. **Language Layers** (built on demand)
   - 🦀 **Rust**: cargo, clippy, rustfmt
   - 🐹 **Go**: go toolchain
   - 🟢 **Node.js**: npm, yarn, pnpm
   - 🐍 **Python**: pip, black, pylint, mypy, pytest
   - ☕ **Java**: OpenJDK, Maven, Gradle
   - ❄️ **Nix**: Nix package manager with flakes
   - 🐘 **PHP**: PHP 8.2, Composer, PHPUnit, PHPStan
   - 🔧 **C/C++**: GCC, Clang, CMake, vcpkg, GDB, Valgrind
   - 🎯 **C#**: .NET SDK 8.0, dotnet-format, EF Core tools
   - 🏗️ **Terraform**: Terraform CLI, tflint
   - ☸️ **Kubernetes**: kubectl, helm, k9s

3. **Agent Layers** (optional)
   - 🤖 **Claude Code**: `claude` CLI
   - 🦾 **GitHub Copilot**: `copilot` CLI
   - ➡️ **Cursor**: `cursor-agent` CLI
   - 🔮 **Gemini**: `gemini` CLI
   - 💻 **Codex**: `codex` CLI

### Image Tagging Strategies

**Shared Mode (Default)**: Layer-based tags for efficient storage

```bash
# Base only
localhost/jail-ai-base:latest

# Base + Rust
localhost/jail-ai-agent-claude:base-rust

# Base + Rust + Node.js + Claude
localhost/jail-ai-agent-claude:base-rust-nodejs
```

**Isolated Mode**: Workspace-specific images

```bash
jail-ai claude --isolated  # Uses: localhost/jail-ai-agent-claude:abc12345
```

## 🔐 Authentication & Configuration

### AI Agent Authentication

**Default Behavior (Minimal Auth)**:

- `jail-ai claude` → Auto-mounts `~/.claude/.credentials.json` (API keys only)
- `jail-ai copilot` → No auth mounted (use `--copilot-dir`)
- `jail-ai cursor` → No auth mounted (use `--cursor-dir`)
- `jail-ai gemini` → No auth mounted (use `--gemini-dir`)
- `jail-ai codex` → No auth mounted (use `--codex-dir`)

**Opt-in Mounting**:

- `--claude-dir`: Mount entire `~/.claude` directory (settings, history)
- `--copilot-dir`: Mount `~/.config/.copilot` directory
- `--cursor-dir`: Mount `~/.cursor` and `~/.config/cursor` directories
- `--gemini-dir`: Mount `~/.gemini` directory
- `--codex-dir`: Mount `~/.codex` directory
  - **First Run**: When `--codex-dir` (or `--agent-configs`) is specified and credentials are missing, automatically enters auth mode
  - **Manual Auth**: Use `--auth` to re-authenticate or update credentials (joins running container or starts stopped one)
- `--jules-dir`: Mount `~/.config/jules` directory
  - **First Run**: When `--jules-dir` (or `--agent-configs`) is specified and credentials are missing, automatically enters auth mode
  - **Manual Auth**: Use `--auth` to re-authenticate or update credentials
- `--agent-configs`: Mount all of the above

### Git and GPG Configuration

Use `--git-gpg` flag to enable:

**Git Configuration**:

- Mounts local `.git/config` or creates `.gitconfig` from your git settings
- Includes: `user.name`, `user.email`, `user.signingkey`
- Enables GPG signing: `commit.gpgsign`, `tag.gpgsign`

**GPG Configuration**:

- Mounts `~/.gnupg` directory
- Auto-mounts all GPG agent sockets:
  - `S.gpg-agent` - Main GPG agent socket
  - `S.gpg-agent.ssh` - SSH authentication socket
  - `S.gpg-agent.extra` - Extra GPG agent socket
  - `S.gpg-agent.browser` - Browser GPG agent socket
- Supports SSH-based GPG signing (`gpg.format=ssh`)

```bash
# Claude with full config + git/GPG
jail-ai claude --claude-dir --git-gpg -- chat "make a commit"
```

## 🛠️ Development

### Build

```bash
# Debug build
cargo build
make build

# Release build
cargo build --release

# Run
cargo run -- <args>
make run ARGS="<args>"
```

### Testing

```bash
# Run all tests
cargo test
make test

# Run specific test
cargo test <test_name>

# Lint
cargo clippy -- -D warnings
make clippy

# Format
cargo fmt
make fmt
```

### Container Images

```bash
# Build custom AI agent image
make build-image

# Test image
make test-image

# Clean image
make clean

# Custom image name/tag
make build-image IMAGE_NAME=custom-name IMAGE_TAG=version
```

## 📚 Documentation

- [CLAUDE.md](CLAUDE.md) - Claude Code guidelines for this project
- [docs/specs/](docs/specs/) - Technical specifications and implementation details
  - [IMAGE_TAGGING_STRATEGY.md](docs/specs/IMAGE_TAGGING_STRATEGY.md) - Image naming and tagging strategy
  - [LAYERED_IMAGES_SUMMARY.md](docs/specs/LAYERED_IMAGES_SUMMARY.md) - Layered image system overview
  - [NIX_FLAKES_SUPPORT.md](docs/specs/NIX_FLAKES_SUPPORT.md) - Nix flakes integration
  - [GIT_CONFIG_VERIFICATION_REPORT.md](docs/specs/GIT_CONFIG_VERIFICATION_REPORT.md) - Git config mapping details
  - [IMPLEMENTATION_SUMMARY.md](docs/specs/IMPLEMENTATION_SUMMARY.md) - Implementation details
- [docs/](docs/) - Man pages and additional documentation

## 🎨 Shell Features

The container uses **zsh** as the default shell with:

- **Powerlevel10k (p10k)** - Fast, minimal theme with git integration
- **fzf** integration:
  - `Ctrl+R` - Search command history
  - `Ctrl+T` - Search files in current directory
  - `Alt+C` - Change to subdirectory
- **Smart history** - 10000 entries with deduplication
- **Useful aliases** - `ll` for detailed listing, colored ripgrep

## 🤝 Contributing

Contributions are welcome! Please follow these guidelines:

1. Use conventional commits with emoji to distinguish commit types
2. Run `cargo clippy` and `cargo fmt` before committing
3. Add tests for new features
4. Update documentation as needed
5. Build before committing

## 📝 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## 🙏 Acknowledgments

- Built with Rust 🦀
- Powered by podman containers
- Inspired by the need for secure AI agent sandboxing

## 📧 Contact

- Repository: https://github.com/cyrinux/jail-ai
- Author: Cyril Levis <git@levis.name>
- Author: Max Baz <max@baz.nu>

---

Made with ❤️ and Rust
