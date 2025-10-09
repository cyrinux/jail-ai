# Upgrade Detection Implementation Summary

## Overview

Enhanced jail-ai with automatic upgrade detection that checks for outdated layers and container image mismatches when re-entering existing containers. This ensures a smooth experience after upgrading the jail-ai binary.

## Changes Made

### 1. `src/backend/podman.rs`

**Added:**
- `get_container_image()` - Public async function to retrieve the current image used by a container

```rust
pub async fn get_container_image(&self, name: &str) -> Result<String>
```

### 2. `src/image_layers.rs`

**Added:**
- `get_expected_image_name()` - Determines what image should be used based on current project state without building it
- `check_layers_need_rebuild()` - Checks if any layers need rebuilding for a given workspace and agent

```rust
pub async fn get_expected_image_name(
    workspace_path: &Path,
    agent_name: Option<&str>,
    isolated: bool,
) -> Result<String>

pub async fn check_layers_need_rebuild(
    workspace_path: &Path,
    agent_name: Option<&str>,
) -> Result<Vec<String>>
```

**How it works:**
- Compares embedded Containerfile hashes with existing layer images
- Detects base layer, language layers, and agent layer changes
- Returns list of outdated layer names

### 3. `src/agent_commands.rs`

**Added:**
- `check_container_upgrade_needed()` - Compares container's current image with expected image
- `prompt_force_rebuild()` - Interactive prompt for user to rebuild

**Modified:**
- `run_ai_agent_command()` - Now checks for updates when re-entering existing containers
  - Made `params` mutable to allow setting `force_rebuild` flag
  - Added comprehensive update detection logic
  - Shows clear upgrade prompts with detailed information

**Enhanced upgrade detection flow:**
1. Check if layers need rebuilding
2. Check if container image is outdated
3. Display comprehensive prompt with:
   - List of outdated layers
   - Container image mismatch details
   - Clear recommendations
4. If user accepts, automatically enable `--force-rebuild`

### 4. `src/main.rs`

**Fixed:**
- Changed `unwrap_or(cwd.clone())` to `unwrap_or_else(|| cwd.clone())` to avoid unnecessary clones (clippy optimization)

### 5. `CLAUDE.md`

**Updated:**
- Added comprehensive documentation for Container Upgrade Detection feature
- Included example prompts and common scenarios
- Updated Key Features section

## Clippy Fixes Applied

1. **Lazy evaluation of clones**: Changed `unwrap_or(cwd.clone())` to `unwrap_or_else(|| cwd.clone())` in 3 locations
   - `src/agent_commands.rs` (2 occurrences)
   - `src/main.rs` (1 occurrence)

## No Dead Code

All added functions are actively used:
- ✅ `get_container_image()` - Called by `check_container_upgrade_needed()`
- ✅ `get_expected_image_name()` - Called by `check_container_upgrade_needed()`
- ✅ `check_layers_need_rebuild()` - Called by `run_ai_agent_command()`
- ✅ `check_container_upgrade_needed()` - Called by `run_ai_agent_command()`
- ✅ `prompt_force_rebuild()` - Called by `run_ai_agent_command()`

## Example User Experience

### Scenario: User upgrades jail-ai binary and runs `jail-ai claude`

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

💡 Recommendation: Use --force-rebuild to:
  • Rebuild outdated layers with latest definitions
  • Recreate container with the correct image
  • Ensure you have the latest tools and security patches

Your data in /home/agent will be preserved during the rebuild.

Would you like to rebuild now? (y/N): y
```

Type `y` → Automatic `--force-rebuild` is triggered, rebuilding all outdated layers and recreating the container.

## Benefits

1. **Automatic Detection**: No manual checking required
2. **Smooth Binary Upgrades**: Detects when embedded Containerfiles change
3. **Clear Communication**: Users understand exactly what needs updating and why
4. **Data Preservation**: `/home/agent` persistent volumes keep user data safe
5. **Optional**: Users can decline and continue with existing container
6. **Comprehensive**: Checks both layers AND container images

## Testing Recommendations

1. Test with existing container after modifying a Containerfile
2. Test with existing container in an unchanged state (should show no updates)
3. Test declining the rebuild prompt
4. Test accepting the rebuild prompt
5. Verify data persistence after rebuild
6. Test with both isolated and shared image modes
7. Test with different project types (rust, nodejs, multi-language, etc.)

## Files Modified

- `src/backend/podman.rs` (+8 lines)
- `src/image_layers.rs` (+68 lines)
- `src/agent_commands.rs` (+90 lines, modified function signature)
- `src/main.rs` (1 line change for clippy)
- `CLAUDE.md` (documentation updates)

## Compilation Status

✅ Code compiles without errors
✅ No clippy warnings (lazy clone evaluation fixed)
✅ No dead code
✅ All functions properly used
✅ All imports necessary

## Future Enhancements

Potential improvements for future consideration:
- Add `--auto-upgrade` flag to skip prompt and always rebuild
- Add `--no-upgrade-check` flag to skip upgrade detection
- Show estimated rebuild time
- Cache upgrade check results for a short duration to avoid repeated checks
- Add metrics/telemetry for upgrade acceptance rate
