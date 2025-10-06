# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`jail-ai` is a Rust-based jail wrapper for sandboxing AI agents (Claude, Copilot, Cursor) using systemd-nspawn or podman backends. It provides isolation, resource limits, and workspace management for secure AI agent execution.

## Commands

### Build and Development
- **Build jail-ai**: `cargo build` or `make build`
- **Build release**: `cargo build --release`
- **Run**: `cargo run -- <args>` or `make run ARGS="<args>"`
- **Run tests**: `cargo test` or `make test`
- **Run single test**: `cargo test <test_name>`
- **Lint**: `cargo clippy -- -D warnings` or `make clippy`
- **Format**: `cargo fmt` or `make fmt`

### Container Image
- **Build custom AI agent image**: `make build-image`
  - Image includes: bash, ripgrep, cargo, go, node, python, and common dev tools
  - Default image name: `localhost/jail-ai-env:latest`
  - Customize with: `make build-image IMAGE_NAME=custom-name IMAGE_TAG=version`
- **Test image**: `make test-image`
- **Clean image**: `make clean`

### Usage Examples
```bash
# Build the custom development image first
make build-image

# Create jail with auto-mounted workspace (uses custom image by default)
cargo run -- create my-agent

# Create jail with specific image
cargo run -- create my-agent --image alpine:latest

# Create jail without workspace mount
cargo run -- create my-agent --no-workspace

# Create jail with entire ~/.claude directory (default: only .claude.json)
cargo run -- create my-agent --claude-dir

# Create jail with ~/.config directory for GitHub Copilot
cargo run -- create my-agent --copilot-dir

# Create jail with ~/.cursor directory for Cursor Agent
cargo run -- create my-agent --cursor-dir

# Create jail with all agent config directories (combines --claude-dir, --copilot-dir, --cursor-dir)
cargo run -- create my-agent --agent-configs

# Create jail with git and GPG configuration mapping
cargo run -- create my-agent --git-gpg

# Create jail with specific agent config and git/GPG support
cargo run -- create my-agent --claude-dir --git-gpg

# Create jail with all config directories and git/GPG support
cargo run -- create my-agent --agent-configs --git-gpg

# Create jail with custom workspace path
cargo run -- create my-agent --workspace-path /app

# Execute command in jail (non-interactive)
cargo run -- exec my-agent -- ls -la /workspace

# Execute command in jail (interactive shell)
cargo run -- exec my-agent --interactive -- bash

# Quick development jail
make dev-jail
```

### Version Management
- Version is managed in `Cargo.toml` and should follow semantic versioning
- Auto-bump version when making changes according to semver rules

## Code Style

- Prefer functional programming patterns in Rust
- Add debug logging where appropriate
- Ensure clippy passes without errors
- Add and update tests as you progress through changes

## Architecture

- **backend/**: Trait-based abstraction with systemd-nspawn and podman implementations
- **cli.rs**: CLI interface using clap
- **config.rs**: Jail configuration with serialization support
- **jail.rs**: High-level jail manager with builder pattern
- **error.rs**: Error types and Result alias

## Key Features

- **Custom Development Image**: Pre-built container with bash, ripgrep, cargo, go, node, python, and essential dev tools
- **AI Agent Integration**: Claude Code, GitHub Copilot CLI, and Cursor Agent pre-installed with minimal auth mounting by default
- **Workspace Auto-mounting**: Current working directory is automatically mounted to `/workspace` in the jail (configurable)
- **Environment Inheritance**: Automatically inherits `TERM` and timezone (`TZ`) from host environment
- **Minimal Auth Mounting**: By default, only minimal auth files (`.claude.json`) are mounted for authentication
- **Granular Config Mounting**: Use `--claude-dir` for `~/.claude`, `--copilot-dir` for `~/.config`, `--cursor-dir` for `~/.cursor`, or `--agent-configs` for all
- **Opt-in Git/GPG Mapping**: Use `--git-gpg` to enable git configuration (name, email, signing key) and GPG config (`~/.gnupg`) mounting
- **Dual Backend Support**: systemd-nspawn (Linux containers) and podman (OCI containers)
- **Resource Limits**: Memory and CPU quota restrictions
- **Network Isolation**: Configurable network access (disabled, private, or shared)
- **Bind Mounts**: Support for read-only and read-write mounts

## Custom Image Tools

The `localhost/jail-ai-env:latest` image includes:
- **Shell**: zsh (default with Powerlevel10k theme), bash
- **Shell Enhancements**:
  - **fzf** - Fuzzy finder for command history and file search
  - **Powerlevel10k** - Beautiful and fast zsh theme with git integration
- **Search**: ripgrep, fd-find
- **Languages**: Rust (cargo, clippy, rustfmt), Go, Node.js, Python 3
- **Build tools**: gcc, make, pkg-config
- **Utilities**: git, vim, nano, jq, tree, tmux, htop, gh (GitHub CLI)
- **Python tools**: black, pylint, mypy, pytest
- **Rust tools**: clippy, rustfmt
- **AI Coding Agents**:
  - **Claude Code** (`claude`) - Anthropic's CLI coding assistant
  - **GitHub Copilot CLI** (`copilot`) - GitHub's AI pair programmer
  - **Cursor Agent** (`cursor-agent`) - Cursor's terminal AI agent

### AI Agent Authentication

The AI coding agents require authentication. By default, **minimal auth files** are mounted:
- **Claude Code**: `~/.claude.json` → `/home/agent/.claude.json` (stores API keys)

**Opt-in mounting** (use flags to enable):
- `--claude-dir`: Mount entire `~/.claude` → `/home/agent/.claude` directory (settings, commands, history)
- `--copilot-dir`: Mount `~/.config` → `/home/agent/.config` directory (GitHub Copilot authentication tokens)
- `--cursor-dir`: Mount `~/.cursor` → `/home/agent/.cursor` directory (Cursor Agent authentication and settings)
- `--agent-configs`: Mount all of the above (combines `--claude-dir`, `--copilot-dir`, `--cursor-dir`)

This means:
- **Default**: Minimal authentication, no extra files exposed to the jail
- **With flags**: Granular control over which configs to mount

Example aliases for different security levels:
```bash
# Minimal (default): only auth files
alias jail-claude='jail-ai claude'

# Medium: add full Claude config directory
alias jail-claude='jail-ai claude --claude-dir'

# Medium: add only Copilot config (for copilot command)
alias jail-copilot='jail-ai copilot --copilot-dir'

# Full: all configs + git/GPG signing
alias jail-claude='jail-ai claude --agent-configs --git-gpg'
```

### Git and GPG Configuration Mapping

When `--git-gpg` flag is used, jail-ai will:

**Git Configuration:**
1. **Local Git Config**: If a `.git/config` file exists in the current directory, it will be mounted to `/home/agent/.gitconfig` in the jail
2. **Global Git Config Fallback**: If no local git config exists, it will read your global git configuration and set environment variables:
   - `GIT_AUTHOR_NAME` and `GIT_COMMITTER_NAME` from `git config --global user.name`
   - `GIT_AUTHOR_EMAIL` and `GIT_COMMITTER_EMAIL` from `git config --global user.email`
   - `GIT_SIGNING_KEY` from `git config --global user.signingkey`

**GPG Configuration:**
- Mount your `~/.gnupg` directory to `/home/agent/.gnupg` in the jail
- This allows GPG signing to work inside the jail using your host's GPG keys

This ensures that git commits made inside the jail will use your configured identity and signing key.

**Note**: Git and GPG configuration mapping are **opt-in** (disabled by default). Use `--git-gpg` flag to enable them.

## Shell Features

The container uses **zsh** as the default shell with:
- **Powerlevel10k (p10k)** - Fast, minimal theme with git integration
- **fzf** integration for command history search (Ctrl+R) and fuzzy completion
- **Smart history** - 10000 entries with deduplication and sharing
- **Useful aliases** - `ll` for detailed listing, colored ripgrep

FZF keybindings:
- `Ctrl+R` - Search command history
- `Ctrl+T` - Search files in current directory
- `Alt+C` - Change to subdirectory

## Git Workflow

- Use conventional commits with emoji to distinguish commit types
- Use `git add -p` for selective staging when appropriate
- Auto-commit when it makes sense (completed features, fixed bugs, etc.)
