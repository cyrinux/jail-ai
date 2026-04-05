# Adding a New AI Agent

This guide walks through every step needed to add a new AI agent to jail-ai.

## Overview

Adding an agent requires changes in **7 files** plus creating **2 new files**. The `Agent` enum in `src/agents/mod.rs` is the central registry. Most other files just wire the new variant through.

| File | What to do |
|------|-----------|
| `src/agents/<name>.rs` | **Create** — agent metadata (constants) |
| `containerfiles/agent-<name>.Containerfile` | **Create** — container image layer |
| `src/agents/mod.rs` | Add variant to `Agent` enum + wire into dispatch macro and match arms |
| `src/image_layers.rs` | Add `include_str!` for the Containerfile + match arm in `get_agent_containerfile` |
| `src/cli.rs` | Add `--<name>-dir` flag to `AgentCommandOptions` and `Create` command, add `Commands` variant |
| `src/main.rs` | Add match arm to dispatch the new `Commands` variant, wire `--<name>-dir` through `Create` |
| `src/agent_commands.rs` | Add `<name>_dir` field to `AgentCommandParams` |

Files that **don't** need changes (already data-driven):
- `src/jail_setup.rs` — uses `Agent::is_config_flag_set()` and `Agent::config_dir_paths()`
- `src/backend/podman.rs` — uses `Agent::from_str()` to detect agent from jail name
- `src/backend/container_app.rs` — same

---

## Step-by-step

### 1. Create the agent module: `src/agents/<name>.rs`

This file defines all metadata for your agent as constants. Copy an existing simple agent (e.g., `opencode.rs`) and modify:

```rust
pub const COMMAND_NAME: &str = "myagent";

pub const NORMALIZED_NAME: &str = "myagent";

pub const DISPLAY_NAME: &str = "MyAgent";

pub const HAS_AUTO_CREDENTIALS: bool = false;

pub const CONFIG_DIR_PATHS: &[(&str, &str)] =
    &[(".config/myagent", "/home/agent/.config/myagent")];

pub const SUPPORTS_AUTH_WORKFLOW: bool = false;

pub const AUTH_CREDENTIAL_PATH: &str = ".config/myagent";
```

**Field reference:**

| Constant | Meaning |
|----------|---------|
| `COMMAND_NAME` | Binary name to execute inside the container (e.g., `claude`, `ccr`, `cursor-agent`) |
| `NORMALIZED_NAME` | Used in jail naming (`jail__project__hash__<name>`) and image tags. Must match `from_str()` |
| `DISPLAY_NAME` | Human-readable name for UI messages |
| `HAS_AUTO_CREDENTIALS` | If `true`, minimal auth (`.credentials.json`) is auto-mounted even without `--<name>-dir` |
| `CONFIG_DIR_PATHS` | Array of `(host_relative_path, container_absolute_path)` pairs. Relative to `$HOME` |
| `SUPPORTS_AUTH_WORKFLOW` | If `true`, `--auth` flag enables interactive OAuth authentication with host networking |
| `AUTH_CREDENTIAL_PATH` | Path (relative to `$HOME`) checked to detect first-run (missing = auto-enable auth) |

**For agents that need a server started first** (like Claude Code Router), also add:

```rust
pub const REQUIRES_SERVER_START: bool = true;
pub const SERVER_START_COMMAND: &str = "start";
pub const MAIN_COMMAND: &str = "code";
```

### 2. Create the Containerfile: `containerfiles/agent-<name>.Containerfile`

```dockerfile
ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with MyAgent CLI"

USER root

# Install your agent (pick the right method)
RUN npm install -g myagent@latest
# or: RUN pip install myagent
# or: RUN curl -fsSL https://example.com/install.sh | bash

USER agent
WORKDIR /workspace

ENV JAIL_AI_AGENT="🤖 MyAgent"

CMD ["/bin/zsh"]
```

### 3. Register in `src/agents/mod.rs`

**3a.** Add the module declaration at the top:

```rust
mod myagent;
```

**3b.** Add variant to the `Agent` enum:

```rust
pub enum Agent {
    // ... existing variants ...
    MyAgent,
}
```

**3c.** Add to `ALL_AGENTS`:

```rust
pub const ALL_AGENTS: &[Agent] = &[
    // ... existing agents ...
    Agent::MyAgent,
];
```

**3d.** Add match arm to `agent_dispatch!` macro:

```rust
macro_rules! agent_dispatch {
    ($self:expr, $field:ident) => {
        match $self {
            // ... existing arms ...
            Self::MyAgent => myagent::$field,
        }
    };
}
```

**3e.** Add to `from_str()`:

```rust
"myagent" => Some(Self::MyAgent),
```

**3f.** Add to `emoji()`:

```rust
Self::MyAgent => "🤖",
```

**3g.** Add to `config_flag_name()`:

```rust
Self::MyAgent => "myagent-dir",
```

**3h.** If your agent needs `requires_server_start` / `server_start_command` / `main_command`, add match arms in those methods.

**3i.** Add to `AgentConfigFlags`:

```rust
pub struct AgentConfigFlags {
    // ... existing fields ...
    pub myagent_dir: bool,
}
```

And in `AgentConfigFlags::get()`:

```rust
Agent::MyAgent => self.myagent_dir,
```

### 4. Add Containerfile to `src/image_layers.rs`

**4a.** Add the `include_str!` constant near the other agent constants:

```rust
const AGENT_MYAGENT_CONTAINERFILE: &str =
    include_str!("../containerfiles/agent-myagent.Containerfile");
```

**4b.** Add match arm in `get_agent_containerfile()`:

```rust
fn get_agent_containerfile(layer: &str) -> Option<&'static str> {
    match layer {
        // ... existing arms ...
        "agent-myagent" => Some(AGENT_MYAGENT_CONTAINERFILE),
        _ => None,
    }
}
```

### 5. Add CLI support in `src/cli.rs`

**5a.** Add `--myagent-dir` flag to `AgentCommandOptions`:

```rust
#[arg(long)]
pub myagent_dir: bool,
```

Also update the `--agent-configs` doc comment to mention `--myagent-dir`.

**5b.** Add the same flag to the `Create` command's fields (search for the block of `xxx_dir` booleans):

```rust
#[arg(long)]
myagent_dir: bool,
```

Also update the `--agent-configs` doc comment in `Create`.

**5c.** Add the `Commands` subcommand variant:

```rust
#[command(name = "myagent")]
MyAgent {
    #[command(flatten)]
    common: AgentCommandOptions,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
},
```

### 6. Wire through `src/main.rs`

**6a.** Add match arm for the new command (search for the agent dispatch block):

```rust
Commands::MyAgent { common, args } => {
    run_agent_command(agents::Agent::MyAgent, common, args, verbose).await?;
}
```

**6b.** In the `Commands::Create` destructuring, add `myagent_dir`:

```rust
Commands::Create {
    // ... existing fields ...
    myagent_dir,
    // ...
} => {
```

**6c.** In the `AgentConfigFlags` construction inside `Create`, add:

```rust
myagent_dir,
```

**6d.** In `run_agent_command()`, add to both `AgentConfigFlags` constructions:

```rust
myagent_dir: common.myagent_dir,
```

### 7. Add to `src/agent_commands.rs`

Add the field to `AgentCommandParams`:

```rust
pub struct AgentCommandParams {
    // ... existing fields ...
    pub myagent_dir: bool,
}
```

And wire it through in the `jail_setup::AgentConfigFlags` construction (search for `opencode_dir: params.opencode_dir`):

```rust
myagent_dir: params.myagent_dir,
```

---

## Checklist

- [ ] `src/agents/<name>.rs` — created with all constants
- [ ] `containerfiles/agent-<name>.Containerfile` — created
- [ ] `src/agents/mod.rs` — module declared, enum variant added, `ALL_AGENTS` updated, `agent_dispatch!` updated, `from_str()` arm added, `emoji()` arm added, `config_flag_name()` arm added, `AgentConfigFlags` field + `get()` arm added
- [ ] `src/image_layers.rs` — `include_str!` added, `get_agent_containerfile()` arm added
- [ ] `src/cli.rs` — `--<name>-dir` in `AgentCommandOptions`, `--<name>-dir` in `Create`, `Commands` variant added, `--agent-configs` docs updated
- [ ] `src/main.rs` — `Commands` match arm added, `Create` destructuring updated, `AgentConfigFlags` wired
- [ ] `src/agent_commands.rs` — `AgentCommandParams` field added, wired through

## Verification

```bash
cargo build
cargo test
cargo clippy -- -D warnings
```

The test `test_all_agents_have_from_str_roundtrip` in `src/agents/mod.rs` will catch the most common mistake: adding an agent to `ALL_AGENTS` but forgetting the `from_str()` match arm.

## Architecture notes

- **`src/agents/mod.rs`** is the central registry. The `agent_dispatch!` macro eliminates per-method match boilerplate — each agent module just exports constants.
- **`src/jail_setup.rs`** is fully data-driven: `mount_agent_configs()` iterates `ALL_AGENTS` and calls `agent.config_dir_paths()`. No per-agent code needed.
- **`src/backend/podman.rs`** and **`src/backend/container_app.rs`** use `Agent::from_str()` to detect agents from jail names. No per-agent code needed.
- **`src/image_layers.rs`** uses `Agent::emoji()` for layer emojis dynamically. Only the Containerfile `include_str!` and `get_agent_containerfile()` need a manual entry (Rust requires `include_str!` at compile time with a literal path).
- The **CLI** (`src/cli.rs`) and **main dispatch** (`src/main.rs`) require explicit enum variants and match arms because clap's `#[derive]` macros work at compile time with concrete types.
