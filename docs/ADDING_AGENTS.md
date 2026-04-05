# Adding a New AI Agent

This guide walks through every step to add a new AI agent to jail-ai.

## Overview

Each agent is defined by **two files** that contain ALL agent-specific logic:

| File | Purpose |
|------|---------|
| `src/agents/<name>.rs` | Agent metadata: binary name, config paths, emoji, containerfile |
| `containerfiles/agent-<name>.Containerfile` | Container image layer |

Then you add **one line per file** in 3 registration points (boilerplate-free wiring):

| File | What to add |
|------|-------------|
| `src/agents/mod.rs` | One entry in `for_each_agent!` macro |
| `src/cli.rs` | One `Commands` variant (5-line copy-paste block) |
| `src/main.rs` | One dispatch line |

**No changes needed** in: `image_layers.rs`, `jail_setup.rs`, `agent_commands.rs`, `backend/podman.rs`, `backend/container_app.rs` — they are fully data-driven.

---

## Step 1: Create `src/agents/<name>.rs`

This is the single source of truth for all agent metadata. Copy and modify:

```rust
pub const COMMAND_NAME: &str = "myagent";
pub const NORMALIZED_NAME: &str = "myagent";
pub const DISPLAY_NAME: &str = "MyAgent";
pub const EMOJI: &str = "🤖";
pub const HAS_AUTO_CREDENTIALS: bool = false;
pub const CONFIG_DIR_PATHS: &[(&str, &str)] =
    &[(".config/myagent", "/home/agent/.config/myagent")];
pub const SUPPORTS_AUTH_WORKFLOW: bool = false;
pub const AUTH_CREDENTIAL_PATH: &str = ".config/myagent";
pub const CLI_ALIASES: &[&str] = &[];
pub const CONTAINERFILE: &str =
    include_str!("../../containerfiles/agent-myagent.Containerfile");
```

### Field reference

| Constant | Meaning |
|----------|---------|
| `COMMAND_NAME` | Binary to execute inside the container (e.g., `claude`, `ccr`, `cursor-agent`) |
| `NORMALIZED_NAME` | Used in jail naming and image tags. Must be parseable by `from_str()` |
| `DISPLAY_NAME` | Human-readable name for UI |
| `EMOJI` | Icon shown in build progress and layer names |
| `HAS_AUTO_CREDENTIALS` | If `true`, `.credentials.json` is auto-mounted without `--config-dir` |
| `CONFIG_DIR_PATHS` | `(host_relative_path, container_absolute_path)` pairs, relative to `$HOME` |
| `SUPPORTS_AUTH_WORKFLOW` | If `true`, `--auth` enables interactive OAuth with host networking |
| `AUTH_CREDENTIAL_PATH` | Path (relative to `$HOME`) checked to detect first-run |
| `CLI_ALIASES` | Alternative names for `from_str()` (e.g., `["ccr"]` for claude-code-router) |
| `CONTAINERFILE` | Embedded containerfile content via `include_str!` |

### Optional: server-start agents

For agents needing a background server (like Claude Code Router), add:

```rust
pub const REQUIRES_SERVER_START: bool = true;
pub const SERVER_START_COMMAND: &str = "start";
pub const MAIN_COMMAND: &str = "code";
```

Then add match arms in `Agent::requires_server_start()`, `server_start_command()`, and `main_command()` in `agents/mod.rs`.

## Step 2: Create `containerfiles/agent-<name>.Containerfile`

```dockerfile
ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with MyAgent CLI"

USER root
RUN npm install -g myagent@latest
USER agent
WORKDIR /workspace

ENV JAIL_AI_AGENT="🤖 MyAgent"
CMD ["/bin/zsh"]
```

## Step 3: Register in `src/agents/mod.rs`

**3a.** Add module declaration:
```rust
mod myagent;
```

**3b.** Add ONE line to `for_each_agent!`:
```rust
#[macro_export]
macro_rules! for_each_agent {
    ($callback:ident) => {
        $callback! {
            // ... existing agents ...
            (MyAgent, myagent, "myagent"),   // ← add this
        }
    };
}
```

This single line auto-generates: enum variant, `ALL_AGENTS` entry, `from_str()` matching, and all dispatched methods (`command_name()`, `display_name()`, `emoji()`, `containerfile()`, etc.).

## Step 4: Add CLI subcommand in `src/cli.rs`

Add to the `Commands` enum (copy-paste block):

```rust
#[command(name = "myagent")]
MyAgent {
    #[command(flatten)]
    common: AgentCommandOptions,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
},
```

## Step 5: Add dispatch in `src/main.rs`

Add ONE line in the agent dispatch block:

```rust
Commands::MyAgent { common, args } => run_agent_command(agents::Agent::MyAgent, common, args, verbose).await?,
```

---

## That's it!

**Total: 2 files created + 3 one-line additions.**

Everything else is data-driven from your agent module:
- Image layer building reads `CONTAINERFILE` from the module
- Layer emoji uses `EMOJI` from the module
- Config mounting uses `CONFIG_DIR_PATHS` from the module
- Agent detection in jail names uses `from_str()` generated from `for_each_agent!`
- Backend image building uses `Agent::from_str()` dynamically

## CLI flags

Users mount agent configs with:
- `--config-dir` — mounts the current agent's config directories
- `--agent-configs` — mounts ALL agents' config directories

No per-agent flags needed.

## Verification

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

The test `test_all_agents_have_from_str_roundtrip` catches missing `for_each_agent!` entries.
The test `test_all_agents_have_containerfile` catches missing containerfiles.
