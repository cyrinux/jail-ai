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
- **Workspace Auto-mounting**: Current working directory is automatically mounted to `/workspace` in the jail (configurable)
- **Dual Backend Support**: systemd-nspawn (Linux containers) and podman (OCI containers)
- **Resource Limits**: Memory and CPU quota restrictions
- **Network Isolation**: Configurable network access (disabled, private, or shared)
- **Bind Mounts**: Support for read-only and read-write mounts

## Custom Image Tools

The `localhost/jail-ai-env:latest` image includes:
- **Shell**: bash
- **Search**: ripgrep, fd-find
- **Languages**: Rust (cargo, clippy, rustfmt), Go, Node.js, Python 3
- **Build tools**: gcc, make, pkg-config
- **Utilities**: git, vim, nano, jq, tree, tmux, htop
- **Python tools**: black, pylint, mypy, pytest
- **Rust tools**: clippy, rustfmt
- **AI Coding Agents**:
  - **Claude Code** (`claude`) - Anthropic's CLI coding assistant
  - **GitHub Copilot CLI** (`copilot`) - GitHub's AI pair programmer
  - **Cursor Agent** (`cursor-agent`) - Cursor's terminal AI agent

### AI Agent Authentication

The AI coding agents require authentication:
- **Claude Code**: Requires Anthropic API key (set `ANTHROPIC_API_KEY`)
- **GitHub Copilot**: Requires GitHub Copilot subscription (use `/login` command)
- **Cursor Agent**: Requires Cursor subscription (authentication via CLI)

## Git Workflow

- Use conventional commits with emoji to distinguish commit types
- Use `git add -p` for selective staging when appropriate
- Auto-commit when it makes sense (completed features, fixed bugs, etc.)
