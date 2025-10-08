# Image Tagging Strategy

## The Problem

When managing multiple projects simultaneously, using `:latest` tags for all images causes conflicts:

```
Project A (~/rust-project-a) → localhost/jail-ai-rust:latest
Project B (~/rust-project-b) → localhost/jail-ai-rust:latest
❌ Both projects share the same image!
```

This breaks isolation and causes issues with:
- Project-specific dependencies
- Different tool versions
- Independent testing
- Parallel development

## The Solution: Hybrid Tagging

We use **two-tier tagging**:
1. **Shared base/language layers**: Tagged with `:latest` (reused across projects)
2. **Final project-specific image**: Tagged with **workspace hash** (unique per project)

### Architecture

```
┌──────────────────────────────────────────────────────┐
│  Project A: localhost/jail-ai-agent-claude:abc12345  │  ← Project-specific
│              (workspace hash: abc12345)              │
└──────────────────────────────────────────────────────┘
                        ▲
                        │ builds from
                        │
┌──────────────────────────────────────────────────────┐
│  Shared: localhost/jail-ai-nodejs:latest             │  ← Shared across projects
│          localhost/jail-ai-rust:latest               │
│          localhost/jail-ai-base:latest               │
└──────────────────────────────────────────────────────┘
```

### Example: Two Rust Projects with Claude

**Project A** (`~/rust-project-a`):
```bash
$ cd ~/rust-project-a
$ jail-ai claude

→ Project hash: abc12345 (from /home/user/rust-project-a)
→ Detected: Rust
→ Building shared layers:
  ✓ localhost/jail-ai-base:latest (shared)
  ✓ localhost/jail-ai-rust:latest (shared)
  ✓ localhost/jail-ai-nodejs:latest (shared)
→ Building project-specific image:
  ✓ localhost/jail-ai-agent-claude:abc12345
→ Container uses: localhost/jail-ai-agent-claude:abc12345
```

**Project B** (`~/rust-project-b`):
```bash
$ cd ~/rust-project-b
$ jail-ai claude

→ Project hash: def67890 (from /home/user/rust-project-b)
→ Detected: Rust
→ Reusing shared layers:
  ✓ localhost/jail-ai-base:latest (cached)
  ✓ localhost/jail-ai-rust:latest (cached)
  ✓ localhost/jail-ai-nodejs:latest (cached)
→ Building project-specific image:
  ✓ localhost/jail-ai-agent-claude:def67890
→ Container uses: localhost/jail-ai-agent-claude:def67890
```

**Result**: Both projects have isolated images but share base layers!

## Image Naming Convention

### Shared Layers (`:latest` tag)
These are **shared** across all projects for efficiency:

| Layer | Image Name | Purpose |
|-------|-----------|---------|
| Base | `localhost/jail-ai-base:latest` | Alpine + common tools |
| Language | `localhost/jail-ai-{rust,golang,python,nodejs,java}:latest` | Language toolchain |

### Project-Specific Images (`:project-hash` tag)
These are **unique** per project directory:

| Type | Image Name | Example |
|------|-----------|---------|
| Language only | `localhost/jail-ai-{lang}:{hash}` | `localhost/jail-ai-rust:abc12345` |
| With agent | `localhost/jail-ai-agent-{agent}:{hash}` | `localhost/jail-ai-agent-claude:abc12345` |

### Hash Generation

The project hash is derived from the **absolute path** of the workspace:

```rust
workspace: /home/user/my-rust-project
→ SHA256 hash of absolute path
→ Take first 8 characters: abc12345
→ Tag: localhost/jail-ai-agent-claude:abc12345
```

This ensures:
- ✅ **Reproducible**: Same directory = same hash
- ✅ **Unique**: Different directories = different hashes
- ✅ **Short**: 8 characters = readable + collision-resistant
- ✅ **Consistent**: Matches container naming (jail-myproject-abc12345-claude)

## Build Flow

### For Agent Commands (e.g., `jail-ai claude`)

```
1. Detect workspace → Generate hash (abc12345)
2. Detect project type → Rust
3. Build shared base:
   → localhost/jail-ai-base:latest
4. Build shared language:
   → localhost/jail-ai-rust:latest
5. Build shared nodejs (for agent):
   → localhost/jail-ai-nodejs:latest
6. Build project-specific agent:
   → localhost/jail-ai-agent-claude:abc12345
   → FROM localhost/jail-ai-nodejs:latest
   → RUN npm install -g @anthropic-ai/claude-code
7. Create container with final image
```

### For Language-Only Projects (e.g., `jail-ai create`)

```
1. Detect workspace → Generate hash (abc12345)
2. Detect project type → Rust
3. Build shared base:
   → localhost/jail-ai-base:latest
4. Build shared language:
   → localhost/jail-ai-rust:latest
5. Tag for project:
   → podman tag localhost/jail-ai-rust:latest \
                localhost/jail-ai-rust:abc12345
6. Create container with tagged image
```

## Benefits

### 1. **Efficient Storage**
Shared layers (base, language) are reused:
```
Project A: 
  - base:latest (200MB, shared)
  - rust:latest (300MB, shared)
  - agent-claude:abc12345 (50MB, unique)
  
Project B:
  - base:latest (reused, 0MB)
  - rust:latest (reused, 0MB)
  - agent-claude:def67890 (50MB, unique)

Total: 550MB (vs 600MB without sharing)
```

### 2. **Fast Rebuilds**
Only rebuild project-specific layer:
```bash
# Update agent for project A
$ cd ~/rust-project-a
$ jail-ai claude --force-rebuild

→ Reuses: base:latest, rust:latest, nodejs:latest
→ Rebuilds only: agent-claude:abc12345 (30 seconds)
```

### 3. **Perfect Isolation**
Each project gets its own final image:
```
Project A → agent-claude:abc12345
Project B → agent-claude:def67890
Project C → agent-copilot:78901234

All isolated, no conflicts! ✓
```

### 4. **Easy Cleanup**
Remove project-specific images without affecting others:
```bash
# Remove project A's image
$ podman rmi localhost/jail-ai-agent-claude:abc12345

# Projects B and C still work
# Shared layers (base, rust, nodejs) still cached
```

## Image Lifecycle

### Creation
```bash
$ jail-ai claude
→ Creates: jail-ai-agent-claude:abc12345
→ Creates: jail-myproject-abc12345-claude (container)
```

### Reuse
```bash
$ jail-ai claude  # Same project, same directory
→ Reuses: jail-ai-agent-claude:abc12345 (no rebuild)
→ Reuses: jail-myproject-abc12345-claude (if exists)
```

### Update
```bash
$ jail-ai claude --force-rebuild
→ Rebuilds: jail-ai-agent-claude:abc12345
→ Recreates: jail-myproject-abc12345-claude
```

### Cleanup
```bash
# Remove container and project-specific image
$ jail-ai remove
→ Removes: jail-myproject-abc12345-claude
→ Removes: jail-ai-agent-claude:abc12345

# Shared layers remain for other projects
```

## Example: Multi-Project Workflow

```bash
# Project 1: Rust + Claude
$ cd ~/rust-project-1
$ jail-ai claude
→ Builds: base, rust, nodejs
→ Creates: agent-claude:a1b2c3d4

# Project 2: Rust + Copilot (reuses base, rust, nodejs)
$ cd ~/rust-project-2
$ jail-ai copilot --copilot-dir
→ Reuses: base, rust, nodejs
→ Creates: agent-copilot:e5f6g7h8

# Project 3: Python + Claude (reuses base, nodejs)
$ cd ~/python-project
$ jail-ai claude
→ Reuses: base, nodejs
→ Builds: python
→ Creates: agent-claude:i9j0k1l2

# List all images
$ podman images | grep jail-ai
jail-ai-base                latest    200MB
jail-ai-rust                latest    500MB
jail-ai-python              latest    280MB
jail-ai-nodejs              latest    250MB
jail-ai-agent-claude        a1b2c3d4  300MB  # Project 1
jail-ai-agent-copilot       e5f6g7h8  300MB  # Project 2
jail-ai-agent-claude        i9j0k1l2  330MB  # Project 3
```

## Matching Container Names

The image tags match the hash in container names:

```
Container: jail-myproject-abc12345-claude
Image:     localhost/jail-ai-agent-claude:abc12345
           └──────────────────┬──────────────────┘
                          Same hash!
```

This makes debugging easier:
```bash
$ podman ps
CONTAINER ID  IMAGE                                   NAME
...           jail-ai-agent-claude:abc12345           jail-myproject-abc12345-claude
```

## Summary

✅ **Shared base layers**: `:latest` tag for efficiency  
✅ **Project-specific finals**: `:workspace-hash` tag for isolation  
✅ **Storage efficient**: Reuse shared layers  
✅ **Rebuild fast**: Only rebuild project layer  
✅ **Perfect isolation**: Each project has unique final image  
✅ **Easy cleanup**: Remove per-project images safely  
✅ **Consistent naming**: Image tag matches container hash  

**Best of both worlds**: Share infrastructure, isolate projects! 🎯
