# Jail-AI Layered Container Images

This directory contains modular Containerfiles for building jail-ai images on-demand based on project type.

## Architecture

The image system is organized into **layers** that build on top of each other:

```
┌─────────────────────────────────────────┐
│  Agent Layer (agent-claude, etc.)       │
│  ├─ Claude Code                         │
│  ├─ Copilot CLI                         │
│  ├─ Cursor Agent                        │
│  ├─ Gemini CLI                          │
│  └─ Codex CLI                           │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│  Language Layer (optional)              │
│  ├─ Rust (cargo, clippy, rustfmt)       │
│  ├─ Go (go toolchain)                   │
│  ├─ Python (pip, black, pytest)         │
│  ├─ Node.js (npm, yarn, pnpm)           │
│  └─ Java (JDK, Maven, Gradle)           │
└─────────────────────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────┐
│  Base Layer (base.Containerfile)        │
│  ├─ Alpine Linux 3.20 (~5MB base)       │
│  ├─ Common tools (git, vim, ripgrep)    │
│  ├─ Shell enhancements (zsh, fzf, p10k) │
│  └─ GitHub CLI                          │
└─────────────────────────────────────────┘
```

## Containerfiles

### Base Layer
- **`base.Containerfile`** - Alpine Linux base with common development tools
  - Image name: `localhost/jail-ai-base:latest`
  - Size: ~200MB (vs ~1GB for Debian-based monolithic image)
  - Includes: bash, zsh, git, vim, ripgrep, fzf, GitHub CLI, etc.

### Language Layers
Each language layer builds on top of the base layer:

- **`golang.Containerfile`** - Go development environment
  - Image name: `localhost/jail-ai-golang:latest`
  - Adds: Go 1.23.4 toolchain

- **`rust.Containerfile`** - Rust development environment
  - Image name: `localhost/jail-ai-rust:latest`
  - Adds: Rust stable, Cargo, Clippy, Rustfmt

- **`python.Containerfile`** - Python development environment
  - Image name: `localhost/jail-ai-python:latest`
  - Adds: Python 3, pip, black, pylint, mypy, pytest, poetry

- **`nodejs.Containerfile`** - Node.js development environment
  - Image name: `localhost/jail-ai-nodejs:latest`
  - Adds: Node.js LTS, npm, yarn, pnpm

- **`java.Containerfile`** - Java development environment
  - Image name: `localhost/jail-ai-java:latest`
  - Adds: OpenJDK 21, Maven, Gradle

### Agent Layers
Agent layers require Node.js, so they build on top of the nodejs layer:

- **`agent-claude.Containerfile`** - Claude Code AI assistant
  - Image name: `localhost/jail-ai-agent-claude:latest`
  - Adds: @anthropic-ai/claude-code

- **`agent-copilot.Containerfile`** - GitHub Copilot CLI
  - Image name: `localhost/jail-ai-agent-copilot:latest`
  - Adds: @github/copilot

- **`agent-cursor.Containerfile`** - Cursor Agent CLI
  - Image name: `localhost/jail-ai-agent-cursor:latest`
  - Adds: cursor-agent

- **`agent-gemini.Containerfile`** - Gemini CLI
  - Image name: `localhost/jail-ai-agent-gemini:latest`
  - Adds: @google/gemini-cli

- **`agent-codex.Containerfile`** - Codex CLI
  - Image name: `localhost/jail-ai-agent-codex:latest`
  - Adds: @openai/codex

## Auto-Detection

When you create a jail, jail-ai automatically detects your project type and builds only the necessary layers:

| Project File(s) | Detected Type | Image Built |
|----------------|---------------|-------------|
| `Cargo.toml` | Rust | `base` → `rust` |
| `go.mod`, `go.sum` | Go | `base` → `golang` |
| `package.json` | Node.js | `base` → `nodejs` |
| `requirements.txt`, `pyproject.toml` | Python | `base` → `python` |
| `pom.xml`, `build.gradle` | Java | `base` → `java` |
| Multiple files | Multi-language | `base` → all detected layers |
| No specific files | Generic | `base` only |

For agent commands (e.g., `jail-ai claude`), the appropriate agent layer is added:
- `base` → `nodejs` → `agent-claude`

## On-Demand Building

Images are built **lazily** (on-demand) when needed:

1. First run: Builds only required layers
2. Subsequent runs: Reuses cached layers
3. Changed Containerfile: Rebuilds only affected layers

Example workflow for a Rust project:
```bash
# First run: builds base + rust layers
cargo run -- create my-rust-project
# → Builds: localhost/jail-ai-base:latest
# → Builds: localhost/jail-ai-rust:latest

# Second run: uses cached images
cargo run -- create my-rust-project
# → Uses existing images (instant)

# Using Claude: adds agent layer
cargo run -- claude
# → Uses: localhost/jail-ai-base:latest (cached)
# → Uses: localhost/jail-ai-nodejs:latest (cached or new)
# → Builds: localhost/jail-ai-agent-claude:latest
```

## Benefits

### 🚀 Faster Startup
- No need to build all languages if you're only using Rust
- Each layer is cached independently
- Incremental builds when only one layer changes

### 💾 Smaller Images
- Base image: ~200MB (Alpine vs ~500MB Debian)
- Language layers: +50-150MB each
- Only install what you need

### 🔧 Better Modularity
- Easy to add new languages
- Easy to update individual layers
- Independent caching per layer

### ⚡ Efficient Rebuilds
- Changed base? Rebuild all layers
- Changed rust layer? Only rebuild rust (not python, go, etc.)
- Changed project? Auto-detect and build appropriate stack

## Manual Usage

You can also manually build specific layers:

```bash
# Build base image
podman build -t localhost/jail-ai-base:latest -f containerfiles/base.Containerfile containerfiles/

# Build rust layer (requires base)
podman build -t localhost/jail-ai-rust:latest --build-arg BASE_IMAGE=localhost/jail-ai-base:latest -f containerfiles/rust.Containerfile containerfiles/

# Build claude agent (requires nodejs)
podman build -t localhost/jail-ai-agent-claude:latest --build-arg BASE_IMAGE=localhost/jail-ai-nodejs:latest -f containerfiles/agent-claude.Containerfile containerfiles/
```

## Customization

To customize a layer:

1. Edit the Containerfile in this directory
2. Rebuild using `cargo run -- create --force-rebuild`
3. Changes are automatically detected

## Legacy Monolithic Image

The old monolithic `Containerfile` in the project root still exists for reference and backward compatibility. To use it instead of layered images:

```bash
# Disable layered images (not yet exposed in CLI)
# For now, custom images bypass the layered system
cargo run -- create --image localhost/my-custom-image:latest
```

## Image Sizes (Approximate)

| Image | Size | Layers |
|-------|------|--------|
| `base` | ~200MB | Alpine + tools |
| `golang` | ~300MB | base + Go |
| `rust` | ~500MB | base + Rust |
| `python` | ~280MB | base + Python |
| `nodejs` | ~250MB | base + Node.js |
| `java` | ~400MB | base + JDK |
| `agent-claude` | ~300MB | nodejs + Claude |

Compare to monolithic: **~1.2GB** with all languages + all agents!
