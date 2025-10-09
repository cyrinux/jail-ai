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

# Create jail with entire ~/.claude directory (default: only .claude/.credentials.json)
cargo run -- create my-agent --claude-dir

# Create jail with ~/.config directory for GitHub Copilot
cargo run -- create my-agent --copilot-dir

# Create jail with ~/.cursor and ~/.config/cursor directories for Cursor Agent
cargo run -- create my-agent --cursor-dir

# Create jail with ~/.config/gemini directory for Gemini CLI
cargo run -- create my-agent --gemini-dir

# Create jail with ~/.config/codex directory for Codex CLI
cargo run -- create my-agent --codex-dir

# Create jail with all agent config directories (combines --claude-dir, --copilot-dir, --cursor-dir, --gemini-dir, --codex-dir)
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

# AI Agent commands with parameters (use -- to separate jail-ai params from agent params)
cargo run -- claude -- chat "help me debug this code"
cargo run -- claude -- --help
cargo run -- claude -- --version
cargo run -- copilot --copilot-dir -- suggest "write tests"
cargo run -- gemini --gemini-dir -- --model gemini-pro "explain this"
# Codex CLI with API key authentication
cargo run -- codex --codex-dir --auth sk-your-key -- generate "create a REST API"

# Codex CLI with manual authentication (interactive shell)
cargo run -- codex --codex-dir --shell

# Start interactive shell in agent jail (without running the agent)
cargo run -- claude --shell
cargo run -- copilot --copilot-dir --shell

# Layer-based (shared) vs Isolated images
# By default, jail-ai uses layer-based tagging for image sharing across projects
cargo run -- claude  # Uses shared image: localhost/jail-ai-agent-claude:base-rust-nodejs

# Use --isolated flag for project-specific images (workspace hash)
cargo run -- claude --isolated  # Uses isolated image: localhost/jail-ai-agent-claude:abc12345
```

### Container Upgrade Detection

When you re-enter an existing container, jail-ai automatically checks if a newer version of the underlying image is available. This happens when:

- Base images or dependencies have been updated
- New tools or features have been added to the Containerfiles
- Security patches have been applied
- The `--force-rebuild` flag was used to rebuild layers

If an upgrade is available, you'll see a prompt like:

```
🔄 Container image update available!
  Current image:  localhost/jail-ai-agent-claude:base-rust-nodejs-abc123
  Expected image: localhost/jail-ai-agent-claude:base-rust-nodejs-def456

This could be due to:
  • Updated base images or dependencies
  • New tools or features added
  • Security patches

Would you like to upgrade the container to use the newer image? (y/N):
```

**How it works:**

- The check is automatic when entering an existing container (no `--force-rebuild` needed)
- It compares the container's current image with what would be built based on the latest Containerfiles
- If you choose to upgrade (type `y`), the container is recreated with the new image
- If you decline (type `N` or just press Enter), the existing container continues to run
- Your data in `/home/agent` is preserved via persistent volumes during upgrades

**To force an upgrade without prompting:**

```bash
cargo run -- claude --force-rebuild
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

- **backend/**: Trait-based abstraction with podman implementation
- **cli.rs**: CLI interface using clap
- **config.rs**: Jail configuration with serialization support
- **jail.rs**: High-level jail manager with builder pattern
- **error.rs**: Error types and Result alias

## Key Features

- **Custom Development Image**: Pre-built container with bash, ripgrep, cargo, go, node, python, nix, and essential dev tools
- **AI Agent Integration**: Claude Code, GitHub Copilot CLI, Cursor Agent, Gemini CLI, and Codex CLI pre-installed
- **Nix Flakes Support**: Automatic detection and loading of Nix flakes when `flake.nix` is present in the workspace
- **Automatic Upgrade Detection**: When re-entering an existing container, jail-ai automatically checks if the underlying image has been updated and prompts you to upgrade
- **Workspace Auto-mounting**: Current working directory is automatically mounted to `/workspace` in the jail (configurable)
- **Environment Inheritance**: Automatically inherits `TERM` and timezone (`TZ`) from host environment, sets `EDITOR=vim`, and configures `SSH_AUTH_SOCK` when GPG SSH agent socket is available
- **Minimal Auth Mounting**: Claude agent auto-mounts `~/.claude/.credentials.json` by default; other agents require explicit config flags
- **Granular Config Mounting**: Use `--claude-dir` for `~/.claude`, `--copilot-dir` for `~/.config/.copilot`, `--cursor-dir` for `~/.cursor`, `--gemini-dir` for `~/.config/gemini`, `--codex-dir` for `~/.config/codex`, or `--agent-configs` for all
- **Opt-in Git/GPG Mapping**: Use `--git-gpg` to enable git configuration (name, email, signing key) and GPG config (`~/.gnupg`) mounting
- **Podman Backend**: Uses podman for OCI container management
- **Resource Limits**: Memory and CPU quota restrictions
- **Network Isolation**: Configurable network access (disabled, private, or shared)
- **Bind Mounts**: Support for read-only and read-write mounts

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

### AI Agent Authentication

The AI coding agents require authentication.

**Default behavior (minimal auth):**

- `jail-ai claude` → Auto-mounts `~/.claude/.credentials.json` → `/home/agent/.claude/.credentials.json` (API keys only)
- `jail-ai copilot` → No auth mounted (use `--copilot-dir` to mount `~/.config/.copilot`)
- `jail-ai cursor` → No auth mounted (use `--cursor-dir` to mount `~/.cursor`)
- `jail-ai gemini` → No auth mounted (use `--gemini-dir` to mount `~/.config/gemini`)
- `jail-ai codex` → No auth mounted (use `--codex-dir` to mount `~/.codex`)

**Opt-in mounting** (use flags to enable):

- `--claude-dir`: Mount entire `~/.claude` → `/home/agent/.claude` directory (settings, commands, history)
- `--copilot-dir`: Mount `~/.config/.copilot` → `/home/agent/.config/.copilot` directory (GitHub Copilot authentication and config)
- `--cursor-dir`: Mount `~/.cursor` → `/home/agent/.cursor` and `~/.config/cursor` → `/home/agent/.config/cursor` directories (Cursor Agent authentication, settings, and config)
- `--gemini-dir`: Mount `~/.config/gemini` → `/home/agent/.config/gemini` directory (Gemini CLI authentication and settings)
- `--codex-dir`: Mount `~/.codex` → `/home/agent/.codex` directory (Codex CLI authentication and settings)
  - **Authentication**: Use `--auth <key>` to provide an API key for authentication
  - **Manual authentication**: Run `jail-ai codex --codex-dir --shell` and manually run `codex auth login` inside the jail
- `--agent-configs`: Mount all of the above (combines `--claude-dir`, `--copilot-dir`, `--cursor-dir`, `--gemini-dir`, `--codex-dir`)

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
