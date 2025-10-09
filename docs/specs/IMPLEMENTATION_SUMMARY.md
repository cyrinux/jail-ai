# Implementation Summary: Automatic Image Building

## Overview

Successfully implemented automatic image building with embedded Containerfile and change detection for jail-ai. This feature provides a zero-configuration experience while allowing users to customize their development environment.

## Changes Made

### 1. New Module: `src/image.rs` (257 lines)

Created a comprehensive image management module with the following functionality:

**Key Functions:**
- `ensure_image_available(image_name)` - Main entry point that ensures the default image is built
- `ensure_containerfile_exists()` - Creates config directory and writes embedded Containerfile if not present
- `calculate_file_hash()` - Computes SHA256 hash of Containerfile for change detection
- `has_containerfile_changed()` - Compares current hash with stored hash to detect modifications
- `build_image_from_containerfile()` - Builds image using podman with inherited stdio for progress visibility
- `image_exists()` - Checks if an image exists locally

**Features:**
- Embedded Containerfile using `include_str!` macro
- XDG Base Directory specification compliant (`~/.config/jail-ai/` or `$XDG_CONFIG_HOME/jail-ai/`)
- SHA256 hashing for efficient change detection
- Automatic rebuild when Containerfile changes
- Only manages default image; custom images use pull-only logic

### 2. Updated Backend: `src/backend/podman.rs`

Modified the `create()` method to:
- Import and use the new `image` module
- Check if the requested image is the default image
- If default: use `ensure_image_available()` for automatic building
- If custom: use existing pull logic
- Maintains backward compatibility

### 3. Configuration Files

**`config/Containerfile`** (209 lines)
- Copy of the repository's Containerfile for reference
- Will be copied to `~/.config/jail-ai/Containerfile` on first use

**`config/README.md`** (51 lines)
- Comprehensive documentation on customization
- Explains automatic rebuilding behavior
- Provides manual build commands
- Documents configuration file locations

### 4. Updated Configuration Module: `src/config.rs`

- Import image module
- Use `image::DEFAULT_IMAGE_NAME` constant instead of hardcoded string
- Maintains consistency across codebase

### 5. Updated CLI: `src/cli.rs`

- Added `DEFAULT_IMAGE` constant
- Updated all `default_value` attributes to use the constant
- Ensures single source of truth for default image name

### 6. Updated Main: `src/main.rs`

- Import new `image` module
- Replace hardcoded image name with `image::DEFAULT_IMAGE_NAME`
- Maintains existing functionality

### 7. Documentation Updates

**`CLAUDE.md`:**
- Added automatic building information to Container Image section
- Updated usage examples to remove manual build-image requirement
- Added reference to config/README.md

**`Makefile`:**
- Updated `build-image` target description to note it's optional
- Removed `build-image` dependency from `dev-jail` and `example-create` targets
- Added notes about automatic building

### 8. Version Bump

- Upgraded from v0.16.0 to v0.17.0 (minor version bump for new feature)

## How It Works

### First Run Flow:

1. User runs `jail-ai create` or `jail-ai claude`
2. Image module checks if default image exists
3. If not, config directory is created at `~/.config/jail-ai/`
4. Embedded Containerfile is written to `~/.config/jail-ai/Containerfile`
5. Image is built using podman with progress output shown to user
6. SHA256 hash of Containerfile is stored in `~/.config/jail-ai/.containerfile.sha256`
7. Jail is created using the built image

### Subsequent Runs with Unchanged Containerfile:

1. User runs jail-ai command
2. Image module checks if default image exists (yes)
3. Calculates current Containerfile hash
4. Compares with stored hash (matches)
5. Skips build, proceeds with jail creation

### Runs After Containerfile Modification:

1. User edits `~/.config/jail-ai/Containerfile`
2. User runs jail-ai command
3. Image module calculates new hash
4. Detects mismatch with stored hash
5. Automatically rebuilds image with new Containerfile
6. Updates stored hash
7. Proceeds with jail creation using updated image

## Benefits

1. **Zero Configuration**: Users can start immediately without manual image building
2. **Customizable**: Users can edit `~/.config/jail-ai/Containerfile` to add tools or modify setup
3. **Automatic Updates**: Changes to Containerfile are detected and applied automatically
4. **Efficient**: SHA256 hashing ensures rebuilds only happen when necessary
5. **Transparent**: Build progress is shown to users with inherited stdio
6. **Backward Compatible**: Custom images via `--image` flag still work as before
7. **Clean Separation**: Only manages default image; doesn't interfere with user's custom images

## Testing

- All 15 existing tests pass
- 3 new tests added for image module:
  - `test_embedded_containerfile_not_empty` - Verifies embedded Containerfile
  - `test_default_image_name` - Checks constant value
  - `test_calculate_hash_consistency` - Validates hash calculation
- Clippy passes with no warnings
- Code formatted with rustfmt

## Future Enhancements

Possible improvements for future versions:
1. Support for multiple Containerfile variants (minimal, full, custom)
2. Image layer caching optimization
3. Parallel building for multiple images
4. Integration with container registries for pre-built images
5. Containerfile templates system
6. Build progress indicators with percentage

## Configuration Locations

- **Embedded Containerfile**: Compiled into binary at build time
- **User Containerfile**: `~/.config/jail-ai/Containerfile` (or `$XDG_CONFIG_HOME/jail-ai/Containerfile`)
- **Hash Cache**: `~/.config/jail-ai/.containerfile.sha256`
- **Reference Containerfile**: `config/Containerfile` in repository

## Command Examples

```bash
# First run - automatically builds image
jail-ai create my-agent

# Customize the image
vim ~/.config/jail-ai/Containerfile

# Next run - automatically detects changes and rebuilds
jail-ai create another-agent

# Use custom image (skips automatic build)
jail-ai create custom --image alpine:latest

# Manual build (optional)
make build-image
```

## Compatibility

- **Rust Version**: Works with existing toolchain (edition 2021)
- **Dependencies**: Only added usage of existing `sha2` crate
- **Platforms**: Linux (podman required for building)
- **Backends**: Automatic building only works with podman backend

## Notes

- The automatic building feature only applies to the default image (`localhost/jail-ai-env:latest`)
- Custom images specified via `--image` flag are not automatically built
- If podman is not available, appropriate error messages are shown
- The embedded Containerfile is identical to the repository's Containerfile at build time
- Users can safely customize their copy without affecting other users or installations
