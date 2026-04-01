# Homebrew Formula for jail-ai

This directory contains the Homebrew formula for `jail-ai`.

## For users: installing jail-ai via Homebrew

### Option A – tap this repository directly

```bash
brew tap cyrinux/jail-ai https://github.com/cyrinux/jail-ai
brew install jail-ai
```

### Option B – install from a dedicated tap (recommended long-term)

If a dedicated tap repository (`homebrew-jail-ai`) is published:

```bash
brew tap cyrinux/jail-ai
brew install jail-ai
```

### Prerequisites

jail-ai needs a container runtime. Install at least one of:

| Backend | Install command | Platform |
|---------|-----------------|----------|
| **podman** (recommended) | `brew install podman` | macOS + Linux |
| **apple/container** | `brew install --cask apple/container/container` | macOS only |

After installing podman on macOS you also need to initialise the VM:

```bash
podman machine init
podman machine start
```

## For maintainers: releasing a new version

The CI workflow (`.github/workflows/release.yml`) handles everything automatically when you push a `v*` tag:

1. Builds Linux binaries (`x86_64` + `aarch64`) via `cross`
2. Builds macOS binaries on `macos-15` (arm64) and `macos-13` (x86_64)
3. Assembles Homebrew bottles (`.tar.gz` archives)
4. Updates `homebrew/jail-ai.rb` with the correct `sha256` values
5. Creates a GitHub Release and attaches all binaries + bottles

### Manual release steps (no CI)

```bash
# 1. Bump version in Cargo.toml, then:
VERSION=0.50.0
git tag "v${VERSION}"
git push origin "v${VERSION}"
# CI takes over from here.
```

### Setting up a dedicated Homebrew tap (optional)

Create a repository called `homebrew-jail-ai` under your GitHub organisation:

```bash
mkdir homebrew-jail-ai && cd homebrew-jail-ai
git init
cp ../jail-ai/homebrew/jail-ai.rb Formula/jail-ai.rb
git add Formula/jail-ai.rb
git commit -m "🍺 initial formula"
git remote add origin git@github.com:cyrinux/homebrew-jail-ai.git
git push -u origin main
```

Users can then install with `brew tap cyrinux/jail-ai && brew install jail-ai`.

## Formula details

| Field | Value |
|-------|-------|
| Bottle storage | GitHub Releases (`root_url`) |
| Platforms | arm64_sequoia, arm64_sonoma, ventura, x86_64_linux |
| Build deps | Rust (via Homebrew) |
| Runtime deps | podman (recommended) or apple/container |
| Completions | bash, zsh, fish (generated at build time) |
| Man page | `docs/jail-ai.1` installed into `man1/` |
