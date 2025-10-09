# Nix Flakes Support for jail-ai

## Summary

Added automatic Nix flakes support to jail-ai. When a `flake.nix` file is detected in the workspace, jail-ai will automatically build and use a container image with Nix package manager and flakes enabled.

## Changes Made

### 1. Project Detection (`src/project_detection.rs`)
- Added `Nix` variant to `ProjectType` enum
- Added detection logic for `flake.nix` files
- Updated `language_layer()` to return `"nix"` for Nix projects
- Added test case for Nix project detection

### 2. Container Layer (`containerfiles/nix.Containerfile`)
Created a new Nix development environment Containerfile that:
- Builds on top of the base image
- Installs Nix package manager in multi-user mode
- Enables Nix flakes and experimental features
- Configures Nix for both system-wide and user-specific usage
- Adds Nix environment setup to shell configs (zsh and bash)
- Sets up proper directory permissions for the agent user

### 3. Image Layers System (`src/image_layers.rs`)
- Added `NIX_IMAGE_NAME` constant: `localhost/jail-ai-nix:latest`
- Embedded `NIX_CONTAINERFILE` from the containerfiles directory
- Updated `get_language_image_name()` to handle Nix projects
- Updated `get_containerfile_content()` to include nix layer
- Updated `build_shared_layer()` to support nix image building
- Added test cases for Nix layer functionality

### 4. Documentation Updates
- Updated `CLAUDE.md` to mention Nix support in key features
- Updated `CLAUDE.md` to list Nix in custom image tools
- Updated `containerfiles/README.md` with Nix layer information
- Added Nix to the auto-detection table
- Added Nix image size estimate

## How It Works

### Automatic Detection
When you create a jail in a directory containing `flake.nix`:

```bash
cd /path/to/nix-project
jail-ai create my-nix-jail
```

jail-ai will:
1. Detect the `flake.nix` file
2. Build the base layer (if not cached)
3. Build the nix layer (if not cached): `localhost/jail-ai-nix:latest`
4. Tag it with project-specific hash
5. Start the jail with Nix available

### Using Nix in the Jail
Inside the jail, you can:

```bash
# Enter the jail
jail-ai join my-nix-jail

# Nix commands are available
nix --version

# Use flakes
nix flake show
nix develop     # Enter the flake's devShell
nix build       # Build the flake's default package
```

### With AI Agents
You can use Nix-enabled jails with AI agents:

```bash
# In a Nix project directory
jail-ai claude
```

This will create a jail with:
- Base layer
- Nix layer (for flake.nix support)
- Claude agent layer

## Nix Features

The Nix layer includes:
- **Nix Package Manager**: Latest version with daemon support
- **Flakes Support**: Enabled by default via experimental features
- **Multi-user Installation**: Proper isolation and security
- **Shell Integration**: Automatic Nix environment setup in zsh and bash
- **User Configuration**: Per-user Nix settings in `~/.config/nix/nix.conf`

## Example Use Cases

### 1. Development with Nix Flakes
```bash
# Project with flake.nix
cd my-nix-project
jail-ai create dev-jail

# Inside jail
nix develop
# Your development environment is now loaded from the flake
```

### 2. Building Nix Projects
```bash
# Build a Nix flake project in isolation
jail-ai create build-jail
jail-ai exec build-jail -- nix build
```

### 3. AI Agent with Nix Project
```bash
# Let Claude help with your Nix project
cd nix-project
jail-ai claude "help me understand this flake.nix"
```

## Image Sizes

- **Base layer**: ~200MB (Alpine + common tools)
- **Nix layer**: ~350MB (base + Nix package manager)
- **Total**: ~350MB for Nix-enabled environment

Compare to installing Nix manually in every project: Nix layer is cached and reused!

## Testing

Added comprehensive tests:
- `test_detect_nix_project()` - Verifies flake.nix detection
- `test_get_language_image_name()` - Verifies correct image name for Nix
- `test_get_containerfile_content()` - Verifies nix Containerfile is embedded

Run tests with:
```bash
cargo test
```

## Future Enhancements

Potential improvements:
- Cache Nix store between jail recreations
- Support for `shell.nix` (classic Nix shells)
- Nix-specific resource limits
- Binary cache configuration
- Nix channel management

## Notes

- Nix flakes are **experimental** but widely used in the Nix community
- The Nix daemon runs in the container for proper multi-user support
- Nix store is stored in the container (not persistent between recreations by default)
- For persistent Nix store, use volume mounts

## Compatibility

- Works with all existing jail-ai features
- Compatible with resource limits, network isolation, etc.
- Can be combined with other project types (multi-language detection)
- Works with all AI agent integrations
