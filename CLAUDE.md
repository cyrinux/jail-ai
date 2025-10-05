# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`jail-ai` is a Rust-based jail wrapper for sandboxing AI agents (Claude, Copilot, Cursor) using systemd-nspawn or podman backends. It provides isolation, resource limits, and workspace management for secure AI agent execution.

## Commands

### Build and Development
- **Build**: `cargo build`
- **Build release**: `cargo build --release`
- **Run**: `cargo run -- <args>`
- **Run tests**: `cargo test`
- **Run single test**: `cargo test <test_name>`
- **Lint**: `cargo clippy -- -D warnings`
- **Format**: `cargo fmt`

### Usage Examples
```bash
# Create jail with auto-mounted workspace (default)
cargo run -- create my-agent --backend podman --image alpine:latest

# Create jail without workspace mount
cargo run -- create my-agent --no-workspace

# Create jail with custom workspace path
cargo run -- create my-agent --workspace-path /app

# Execute command in jail
cargo run -- exec my-agent -- ls -la /workspace
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

- **Workspace Auto-mounting**: Current working directory is automatically mounted to `/workspace` in the jail (configurable)
- **Dual Backend Support**: systemd-nspawn (Linux containers) and podman (OCI containers)
- **Resource Limits**: Memory and CPU quota restrictions
- **Network Isolation**: Configurable network access (disabled, private, or shared)
- **Bind Mounts**: Support for read-only and read-write mounts

## Git Workflow

- Use conventional commits with emoji to distinguish commit types
- Use `git add -p` for selective staging when appropriate
- Auto-commit when it makes sense (completed features, fixed bugs, etc.)
