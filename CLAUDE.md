# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`jail-ai` is a Rust-based jail wrapper for sandboxing AI agents (Claude, Copilot, Cursor, Gemini) using podman. It provides isolation, resource limits, and workspace management for secure AI agent execution.

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
- **Automatic building**: jail-ai will automatically build the default image if not present
  - Containerfile is embedded in the binary and copied to `~/.config/jail-ai/Containerfile` on first use
  - Edit `~/.config/jail-ai/Containerfile` to customize the image
  - Changes are detected automatically and the image is rebuilt on next jail creation
  - See `config/README.md` for customization details

### Usage Examples

```bash
# The default image is automatically built if not present
# No need to run make build-image manually anymore!

# Create jail with auto-mounted workspace (uses default image, auto-builds if needed)
cargo run -- create my-agent

# Create jail with specific image
cargo run -- create my-agent --image alpine:latest

# Create jail without workspace mount
cargo run -- create my-agent --no-workspace

# Create jail with port mapping (e.g., PostgreSQL)
cargo run -- create my-agent -p 5432:5432

# Create jail with multiple port mappings
cargo run -- create my-agent -p 8080:80 -p 5432:5432

# Create jail with port mapping using UDP protocol
cargo run -- create my-agent -p 53:53/udp

# Create jail with entire ~/.claude directory (default: only .claude/.credentials.json)
cargo run -- create my-agent --claude-dir

# Create jail with ~/.config directory for GitHub Copilot
cargo run -- create my-agent --copilot-dir

# Create jail with ~/.cursor and ~/.config/cursor directories for Cursor Agent
cargo run -- create my-agent --cursor-dir

# Create jail with ~/.gemini directory for Gemini CLI
cargo run -- create my-agent --gemini-dir

# Create jail with ~/.config/codex directory for Codex CLI
cargo run -- create my-agent --codex-dir

# Create jail with ~/.config/jules directory for Jules CLI
cargo run -- create my-agent --jules-dir

# Create jail with all agent config directories (combines --claude-dir, --copilot-dir, --cursor-dir, --gemini-dir, --codex-dir, --jules-dir)
cargo run -- create my-agent --agent-configs

# Create jail with git and GPG configuration mapping
cargo run -- create my-agent --git-gpg

# Create jail with specific agent config and git/GPG support
cargo run -- create my-agent --claude-dir --git-gpg

# Create jail with all config directories (claude, copilot, cursor, gemini, codex, jules) and git/GPG support
cargo run -- create my-agent --agent-configs --git-gpg

# Create jail with custom workspace path
cargo run -- create my-agent --workspace-path /app

# Create jail skipping nix layer (when flake.nix is present, use other detected languages instead)
cargo run -- create my-agent --no-nix

# Execute command in jail (non-interactive)
cargo run -- exec my-agent -- ls -la /workspace

# Execute command in jail (interactive shell)
cargo run -- exec my-agent --interactive -- bash

# Quick development jail
make dev-jail

# AI Agent commands with parameters (use -- to separate jail-ai params from agent params)
cargo run -- claude -- chat "help me debug this code"
cargo run -- claude -- --help
cargo run -- claude -- --version
cargo run -- copilot --copilot-dir -- suggest "write tests"
cargo run -- gemini --gemini-dir -- --model gemini-pro "explain this"
# Codex CLI - Open interactive shell for OAuth authentication
cargo run -- codex --codex-dir --auth

# Codex CLI - Run agent after authentication is complete
cargo run -- codex --codex-dir -- generate "create a REST API"
cargo run -- jules --jules-dir -- chat "help me debug this code"
cargo run -- jules --jules-dir -- --help

# AI Agent with port mapping (e.g., for connecting to host PostgreSQL)
cargo run -- claude -p 5432:5432 -- chat "help me with database queries"
cargo run -- claude -p 8080:80 -p 5432:5432 -- chat "debug my web app and database"

# AI Agent commands skipping nix layer (use other detected languages instead)
cargo run -- claude --no-nix -- chat "help me debug this code"
cargo run -- copilot --no-nix --copilot-dir -- suggest "write tests"

# Codex CLI with manual authentication (interactive shell)
cargo run -- codex --codex-dir --shell

# Start interactive shell in agent jail (without running the agent)
cargo run -- claude --shell
cargo run -- copilot --copilot-dir --shell
cargo run -- jules --jules-dir --shell

# Layer-based (shared) vs Isolated images
# By default, jail-ai uses layer-based tagging for image sharing across projects
cargo run -- claude  # Uses shared image: localhost/jail-ai-agent-claude:base-rust-nodejs

# Use --isolated flag for project-specific images (workspace hash)
cargo run -- claude --isolated  # Uses isolated image: localhost/jail-ai-agent-claude:abc12345
```

### Container Upgrade Detection

When you re-enter an existing container, jail-ai automatically checks for updates in two areas:

1. **Outdated Layers** - Detects if layer images need rebuilding (e.g., after upgrading jail-ai binary)
2. **Container Image Mismatch** - Detects if the container's image differs from what should be used

This ensures a smooth experience after upgrading your jail-ai binary or when Containerfiles are updated.

**Example prompt when updates are detected:**

```
🔄 Update available for your jail environment!

📦 Outdated layers detected:
  • base
  • rust
  • agent-claude

This typically happens after upgrading the jail-ai binary.
Layers contain updated tools, dependencies, or security patches.

🐳 Container image mismatch:
  Current:  localhost/jail-ai-agent-claude:base-rust-nodejs-abc123
  Expected: localhost/jail-ai-agent-claude:base-rust-nodejs-def456

💡 Recommendation: Use --upgrade to:
  • Rebuild outdated layers with latest definitions
  • Recreate container with the correct image
  • Ensure you have the latest tools and security patches

Your data in /home/agent will be preserved during the rebuild.

Would you like to rebuild now? (y/N):
```

**How it works:**

- The check is automatic when entering an existing container (no `--upgrade` needed)
- Compares embedded Containerfile hashes to detect outdated layers
- Compares the container's current image with what should be used
- If you choose to rebuild (type `y`), it performs a full `--upgrade` automatically
- If you decline (type `N` or just press Enter), the existing container continues to run
- Your data in `/home/agent` is preserved via persistent volumes during rebuilds

**To force a rebuild without prompting:**

```bash
cargo run -- claude --upgrade
```

**Common scenarios:**

- **After upgrading jail-ai binary**: Embedded Containerfiles change, so layers are detected as outdated
- **After `git pull` with Containerfile changes**: Layers with modified definitions are detected
- **After rebuilding specific layers with `--upgrade`**: Container image tag changes, prompting recreation

### Version Management

- Version is managed in `Cargo.toml` and should follow semantic versioning
- Auto-bump version when making changes according to semver rules

## Code Style

- Prefer functional programming patterns in Rust
- Add debug logging where appropriate
- Ensure clippy passes without errors
- Add and update tests as you progress through changes

## Architecture

- **backend/**: Trait-based abstraction with podman implementation
- **cli.rs**: CLI interface using clap
- **config.rs**: Jail configuration with serialization support
- **jail.rs**: High-level jail manager with builder pattern
- **error.rs**: Error types and Result alias

## Key Features

- **Custom Development Image**: Pre-built container with bash, ripgrep, cargo, go, node, python, nix, and essential dev tools
- **AI Agent Integration**: Claude Code, GitHub Copilot CLI, Cursor Agent, Gemini CLI, and Codex CLI pre-installed
- **Nix Flakes Support**: When `flake.nix` is detected, Nix takes precedence and only base + nix + agent layers are used (excluding rust/node/etc). Use `--no-nix` to skip nix and activate other language layers instead
- **Automatic Upgrade Detection**: When re-entering an existing container, jail-ai automatically checks for outdated layers and container image mismatches, prompting you to rebuild. This ensures a smooth experience after upgrading the jail-ai binary.
- **Workspace Auto-mounting**: Current working directory is automatically mounted to `/workspace` in the jail (configurable)
- **Environment Inheritance**: Automatically inherits `TERM` and timezone (`TZ`) from host environment, sets `EDITOR=vim`, and configures `SSH_AUTH_SOCK` when GPG SSH agent socket is available
- **Minimal Auth Mounting**: Claude agent auto-mounts `~/.claude/.credentials.json` by default; other agents require explicit config flags
- **Granular Config Mounting**: Use `--claude-dir` for `~/.claude`, `--copilot-dir` for `~/.config/.copilot`, `--cursor-dir` for `~/.cursor`, `--gemini-dir` for `~/.gemini`, `--codex-dir` for `~/.config/codex`, or `--agent-configs` for all
- **Opt-in Git/GPG Mapping**: Use `--git-gpg` to enable git configuration (name, email, signing key) and GPG config (`~/.gnupg`) mounting
- **Podman Backend**: Uses podman for OCI container management
- **Resource Limits**: Memory and CPU quota restrictions
- **Network Isolation**: Configurable network access (disabled, private, or shared)
- **Bind Mounts**: Support for read-only and read-write mounts

## Custom Project Layer

jail-ai supports project-specific customization through a `jail-ai.Containerfile` in your project root. When this file is present, it will be automatically detected and built as a custom layer in the image stack:

**Build Order**: base → language layers (rust, nodejs, etc.) → **custom** → agent layers (claude, copilot, etc.)

### Creating a Custom Layer

Create a file named `jail-ai.Containerfile` in your project root:

```dockerfile
# jail-ai.Containerfile - Custom layer for this project
ARG BASE_IMAGE
FROM ${BASE_IMAGE}

USER root

# Install project-specific tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    vim-nox \
    && rm -rf /var/lib/apt/lists/*

# Install project-specific npm packages
RUN npm install -g your-package

USER agent
WORKDIR /workspace
```

### Features

- **Automatic Detection**: jail-ai automatically detects `jail-ai.Containerfile` in the project root
- **Layer Caching**: The custom layer is cached and only rebuilt when the Containerfile changes or with `--upgrade`
- **Shared by Default**: In shared mode, projects with the same language stack + custom layer share the image
- **Isolated Mode**: Use `--isolated` flag for project-specific images (uses workspace hash in tag)
- **Force Rebuild**: Use `--upgrade --force-layers custom` to force rebuild of just the custom layer

### Use Cases

- **Project-specific tools**: Install tools that are only needed for this project
- **Custom configurations**: Set up project-specific environment variables or configs
- **Version pinning**: Install specific versions of tools that differ from the base layers
- **Development dependencies**: Add debugging tools or profilers for this project
- **CI/CD alignment**: Match the exact environment used in your CI/CD pipeline

### Image Tag Examples

Without custom layer: `localhost/jail-ai-agent-claude:base-rust-nodejs`  
With custom layer: `localhost/jail-ai-agent-claude:base-rust-nodejs-custom`

See `examples/jail-ai.Containerfile` for a complete template.

## Custom Image Tools

The layered image system automatically detects your project type and builds appropriate images with the following tools:

- **Shell**: zsh (default with Powerlevel10k theme), bash
- **Shell Enhancements**:
  - **fzf** - Fuzzy finder for command history and file search
  - **Powerlevel10k** - Beautiful and fast zsh theme with git integration
- **Search**: ripgrep, fd-find
- **Languages**: 
  - Rust (cargo, clippy, rustfmt)
  - Go (go toolchain)
  - Node.js (npm, yarn, pnpm)
  - Python 3 (pip, black, pylint, mypy, pytest)
  - Java (OpenJDK, Maven, Gradle)
  - Nix (with flakes support)
  - PHP (8.2, Composer, PHPUnit, PHPStan, PHP-CS-Fixer)
  - C/C++ (GCC, Clang, CMake, vcpkg, GDB, Valgrind)
  - C# (.NET SDK 8.0, dotnet-format, EF Core tools)
- **Build tools**: gcc, make, cmake, pkg-config
- **Utilities**: git, vim, nano, helix, jq, tree, tmux, htop, gh (GitHub CLI)
- **Python tools**: black, pylint, mypy, pytest
- **Rust tools**: clippy, rustfmt
- **Nix tools**: Nix package manager with flakes enabled
- **AI Coding Agents**:
  - **Claude Code** (`claude`) - Anthropic's CLI coding assistant
  - **GitHub Copilot CLI** (`copilot`) - GitHub's AI pair programmer
  - **Cursor Agent** (`cursor-agent`) - Cursor's terminal AI agent
  - **Gemini CLI** (`gemini`) - Google's AI terminal assistant
  - **Codex CLI** (`codex`) - OpenAI's Codex CLI for code generation
  - **Jules CLI** (`jules`) - Google's AI coding assistant CLI

### AI Agent Authentication

The AI coding agents require authentication.

**Default behavior (minimal auth):**

- `jail-ai claude` → Auto-mounts `~/.claude/.credentials.json` → `/home/agent/.claude/.credentials.json` (API keys only)
- `jail-ai copilot` → No auth mounted (use `--copilot-dir` to mount `~/.config/.copilot`)
- `jail-ai cursor` → No auth mounted (use `--cursor-dir` to mount `~/.cursor`)
- `jail-ai gemini` → No auth mounted (use `--gemini-dir` to mount `~/.gemini`)
- `jail-ai codex` → No auth mounted (use `--codex-dir` to mount `~/.codex`)
- `jail-ai jules` → No auth mounted (use `--jules-dir` to mount `~/.config/jules`)

**Opt-in mounting** (use flags to enable):

- `--claude-dir`: Mount entire `~/.claude` → `/home/agent/.claude` directory (settings, commands, history)
- `--copilot-dir`: Mount `~/.config/.copilot` → `/home/agent/.config/.copilot` directory (GitHub Copilot authentication and config)
- `--cursor-dir`: Mount `~/.cursor` → `/home/agent/.cursor` and `~/.config/cursor` → `/home/agent/.config/cursor` directories (Cursor Agent authentication, settings, and config)
- `--gemini-dir`: Mount `~/.gemini` → `/home/agent/.gemini` directory (Gemini CLI authentication and settings)
- `--codex-dir`: Mount `~/.codex` → `/home/agent/.codex` directory (Codex CLI authentication and settings)
  - **Authentication**: Use `--auth` flag to open interactive shell for OAuth authentication
    - `jail-ai codex --codex-dir --auth` opens a shell for running `codex auth login`
    - If container is running: joins the running container
    - If container is stopped: starts the container and opens a shell
  - **Security Note**: After authentication, restart the container with `jail-ai codex` to restore secure network isolation
- `--jules-dir`: Mount `~/.config/jules` → `/home/agent/.config/jules` directory (Jules CLI authentication and settings)
  - **Authentication**: Use `--auth` flag to open interactive shell for OAuth authentication
    - `jail-ai jules --jules-dir --auth` opens a shell for running `jules auth`
    - If container is running: joins the running container
    - If container is stopped: starts the container and opens a shell
  - **Security Note**: After authentication, restart the container with `jail-ai jules` to restore secure network isolation
- `--agent-configs`: Mount all of the above (combines `--claude-dir`, `--copilot-dir`, `--cursor-dir`, `--gemini-dir`, `--codex-dir`, `--jules-dir`)

**Note**:
- **OAuth Authentication**: The `--auth` flag provides a convenient way to authenticate agents (Codex, Jules) that require OAuth workflows. It opens an interactive shell in the container where you can run the agent's authentication command. After authentication is complete, restart the container without `--auth` to restore secure network isolation.
- **Automatic Auth Detection**: For agents that support OAuth workflows (Codex, Jules), jail-ai automatically detects when credentials are missing or empty (first run) and enables auth mode automatically **if you've specified the appropriate config directory flag** (`--codex-dir`, `--jules-dir`, or `--agent-configs`). This means on first run with these flags, you don't need to manually specify `--auth` - the system will detect the need for authentication and guide you through the process.

Example aliases for different security levels:

```bash
# Claude: minimal auth by default
alias jail-claude='jail-ai claude'

# Copilot: needs explicit config for auth
alias jail-copilot='jail-ai copilot --copilot-dir'

# Cursor: needs explicit config for auth
alias jail-cursor='jail-ai cursor --cursor-dir'

# Gemini: needs explicit config for auth
alias jail-gemini='jail-ai gemini --gemini-dir'

# Codex: needs explicit config for auth
alias jail-codex='jail-ai codex --codex-dir'

# Jules: needs explicit config for auth
alias jail-jules='jail-ai jules --jules-dir'

# Claude with full config + git/GPG
alias jail-claude-full='jail-ai claude --claude-dir --git-gpg'
```

### Git and GPG Configuration Mapping

When `--git-gpg` flag is used, jail-ai will:

**Git Configuration:**

1. **Local Git Config**: If a `.git/config` file exists in the current directory, it will be mounted to `/home/agent/.gitconfig` in the jail
2. **Project Git Config Fallback**: If no local git config file exists, it will read your project's git configuration (or global as fallback) and create a `.gitconfig` file in the container with the following values:
   - `user.name`, `user.email`, `user.signingkey`
   - `commit.gpgsign`, `tag.gpgsign` - Enables automatic GPG signing for commits and tags
   - `gpg.format`, `gpg.program`, `gpg.ssh.allowedsignersfile` - GPG configuration
   - `core.editor`, `init.defaultbranch`, `pull.rebase`, `push.autosetupremote` - Git behavior settings

**GPG Configuration:**

- Mount your `~/.gnupg` directory to `/home/agent/.gnupg` in the jail
- This allows GPG signing to work inside the jail using your host's GPG keys
- **GPG Agent Sockets**: Automatically mounts all GPG agent sockets from `/run/user/<UID>/gnupg/` including:
  - `S.gpg-agent` - Main GPG agent socket
  - `S.gpg-agent.ssh` - SSH authentication socket (sets `SSH_AUTH_SOCK` environment variable)
  - `S.gpg-agent.extra` - Extra GPG agent socket
  - `S.gpg-agent.browser` - Browser GPG agent socket
- **SSH-based GPG Signing**: If `gpg.format=ssh` is configured, automatically mounts your SSH allowed signers file (`gpg.ssh.allowedsignersfile`) to `/home/agent/.ssh/allowed_signers` in the jail
  - If the SSH allowed signers file doesn't exist, a warning is logged but the jail creation continues
  - SSH GPG signing may not work properly without the allowed signers file
  - Supports both quoted and unquoted git config values (e.g., `"ssh"` or `ssh`)

This ensures that git commits and tags made inside the jail will use your configured identity and signing key.

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
- you have to build before commit
- Never modify my git config.
