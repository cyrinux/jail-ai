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
The Nix development environment Containerfile:
- Builds on top of the base image
- Installs Nix in **single-user mode** (`--no-daemon`), owned by the `agent` user, with its
  profile in `/usr/local/nix-state` (outside the persistent `/home/agent` volume, so switching
  between nix and non-nix images never leaves stale Nix state in `$HOME`)
- Writes `/etc/nix/nix.conf`:
  ```
  experimental-features = nix-command flakes
  max-jobs = auto
  ```
  Nix defaults to `max-jobs = 1` and the single-user installer writes no `nix.conf`, so without
  this line derivations are built one at a time
- Snapshots the installed Nix closure into `/usr/local/nix-seed` (see *Nix Store* below)
- Adds `nix-wrapper` (enters `nix develop` when `/workspace/flake.nix` exists) and zsh/bash
  integration, all of which source `/usr/local/share/jail-ai/nix-init.sh`

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
jail-ai agents claude

# Options belong before the agent name
jail-ai agents --nix-store project claude
```

This will create a jail with:
- Base layer
- Nix layer (for flake.nix support)
- Claude agent layer

## Nix Features

The Nix layer includes:
- **Nix Package Manager**: single-user install, no daemon inside the jail
- **Flakes Support**: enabled system-wide in `/etc/nix/nix.conf`
- **Parallel builds**: `max-jobs = auto`
- **Shell Integration**: automatic Nix environment setup in zsh and bash

## Nix Store

`/nix` is provided according to `--nix-store` (recorded in the `jail-ai.nix-store` label):

| Mode | `/nix` source | Notes |
|------|---------------|-------|
| `shared` (default) | global `jail-ai-nix` volume | all projects reuse downloaded and built paths |
| `project` | `<jail>__nix` volume per project | previous default; jails without the label are treated as `project` |
| `host` | host `/nix/store` (read-only) + host nix-daemon socket | `NIX_REMOTE=daemon`, the host nix client is put on `PATH`; builds run in the host daemon, **with your host user's trust level** — see [Security: host mode](#security-host-mode); Linux Nix hosts only |

Existing jails keep their mode across recreation (`--upgrade`) unless `--nix-store` is
given explicitly, which recreates the container when it differs.

Podman only populates a named volume from the image when the volume is empty, so a shared or
older volume may lack the Nix version of the current image. `nix-init.sh` checks this on
every shell and wrapper start (a single `stat`), and `jail-ai-nix-seed` then copies the missing
paths from `/usr/local/nix-seed`, registers them with `nix-store --load-db` and pins them with a
GC root in `/nix/var/nix/gcroots/jail-ai/`, so `nix-collect-garbage` in one jail never removes
another image's Nix. Removing a jail never removes the `jail-ai-nix` volume.

### Security: host mode

`--nix-store host` widens the sandbox, and by more than the read-only `/nix/store` mount suggests.

The jail runs with `--userns=keep-id`, so the `agent` user maps to your host uid. The nix-daemon
decides trust from the peer credentials of the connection, which means **if your host uid is a
`trusted-users` member of the daemon, so is the jail.** On a default NixOS install
`trusted-users = root @wheel`, and the invoking user is usually in `wheel`, so this is the common
case rather than the exception. Inside such a jail:

```console
$ nix store info
Store URL: daemon
Version: 2.34.8
Trusted: 1
```

A trusted daemon client can set `substituters`, `post-build-hook` and `builders` for daemon
operations, and import arbitrary paths into the host store. A compromised agent can therefore
persist content into the host's `/nix/store` and influence what later host builds fetch and run.

Two things make this easy to misjudge:

- **`nix config show` inside the jail is misleading.** It prints `trusted-users = root`, because it
  reads the *container's* `/etc/nix/nix.conf`. The daemon's own configuration is what applies, and
  the container has no view of it. Check `nix store info` instead — the daemon self-reports.
- **`max-jobs = auto` does not apply in host mode.** It is a client-side setting, and in daemon mode
  the host daemon schedules builds, so the host's `max-jobs` governs. The parallel-build improvement
  applies to `shared` and `project` only.

Use `shared` (the default) for anything you would not trust with your host store. Reserve `host` for
trusting your own agent on your own machine, where sharing the host's already-built paths is worth it.

If you want host store reuse *without* granting trust, run a second daemon with an empty
`trusted-users` on its own socket and point the jail at that:

```bash
sudo env NIX_CONFIG='trusted-users=' \
  nix-daemon --daemon-socket-path /run/nix-untrusted/socket
```

This is a host-side setup, so jail-ai does not automate it; `--nix-store host` currently always uses
`/nix/var/nix/daemon-socket`.

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
jail-ai agents claude -- "help me understand this flake.nix"
```

## Image Sizes

- **Base layer**: ~200MB (Alpine + common tools)
- **Nix layer**: ~350MB (base + Nix package manager), plus ~112MB for the `/usr/local/nix-seed`
  snapshot that lets a stale or shared `/nix` volume be repaired without network access
- **Total**: ~460MB for Nix-enabled environment

Compare to installing Nix manually in every project: Nix layer is cached and reused!

## Testing

Unit tests (`cargo test`):
- `test_detect_nix_project()` - Verifies flake.nix detection
- `test_get_language_image_name()` - Verifies correct image name for Nix
- `test_get_containerfile_content()` - Verifies nix Containerfile is embedded
- `test_nix_store_flag_parsing()` (`tests/cli.rs`) - `--nix-store` parsing and rejection of bad values
- `project_mode_uses_per_project_volume`, `shared_mode_uses_global_volume`,
  `host_mode_mounts_store_and_daemon_socket`, `host_mode_without_daemon_socket_fails`,
  `host_mode_without_nix_client_fails`, `non_nix_image_gets_no_nix_store`,
  `nix_image_uses_shared_store_by_default`, `nix_store_label_round_trip`
  (`src/backend/podman.rs`) - podman argument construction per store mode

### Manual verification

No test in the suite starts a container, so the store modes have to be checked by hand on a Linux
host with podman. Work through the matrix below after changing anything in
`containerfiles/nix.Containerfile`, `nix_run_args()` or the store-mode plumbing.

Setup — two flake projects and one non-nix project:

```bash
JAIL=$PWD/target/release/jail-ai
mkdir -p /tmp/jt/nix-a /tmp/jt/nix-b /tmp/jt/plain
for d in nix-a nix-b; do
  cat > /tmp/jt/$d/flake.nix <<'EOF'
{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  outputs = { nixpkgs, ... }:
    let pkgs = nixpkgs.legacyPackages.x86_64-linux; in {
      devShells.x86_64-linux.default = pkgs.mkShell { packages = [ pkgs.hello ]; };
    };
}
EOF
  (cd /tmp/jt/$d && git init -q && git add flake.nix)
done
printf '[package]\nname="plain"\nversion="0.1.0"\nedition="2021"\n' > /tmp/jt/plain/Cargo.toml

# run a command in a jail with the Nix environment loaded
jx() { local j=$1; shift; podman exec "$j" bash -c "source /usr/local/share/jail-ai/nix.bash && $*"; }
```

| # | Check | Expected |
|---|-------|----------|
| 1 | `cd /tmp/jt/nix-a && $JAIL create nixa --upgrade`, then `podman run --rm --entrypoint cat localhost/jail-ai-nix:latest /etc/nix/nix.conf` | two lines: flakes, `max-jobs = auto` |
| 1 | `podman run --rm --entrypoint sh localhost/jail-ai-nix:latest -c 'cat /usr/local/nix-seed/profile; du -sh /usr/local/nix-seed'` | a `…-user-environment` path, ~110 MB |
| 2 | `cd /tmp/jt/plain && $JAIL create plainj`, then inspect mounts and the `jail-ai.nix-store` label | no `/nix` mount, empty label |
| 2 | `$JAIL agents --nix-store shared claude -- --version` twice in `plain`, comparing `{{.Created}}` | unchanged — a non-nix jail is never recreated over the store mode |
| 3 | `podman inspect nixa --format '{{range .Mounts}}{{.Name}}:{{.Destination}} {{end}}'` | `jail-ai-nix:/nix`, label `shared` |
| 3 | `jx nixa 'nix config show \| grep ^max-jobs'` | `max-jobs = <nproc>` |
| 3 | `jx nixa 'ls /nix/var/nix/gcroots/jail-ai/'` | GC root for the image's Nix |
| 3 | build in one jail, `nix path-info` in another (`nixb`) | resolves instantly, no download |
| 4 | `$JAIL create nixa-proj --nix-store project` | mount `nixa-proj__nix:/nix`, label `project` |
| 5 | wipe the seeded profile + nix package + `gcroots/jail-ai` from a `__nix` volume, recreate, run `nix --version` | prints `🔵 Restoring Nix into /nix/store...` once, then the version; silent on the second run |
| 5 | `jx <jail> 'nix-store --verify-path <profile> <nixpkg>'`, then `nix-collect-garbage` | `ok`; Nix survives the GC and still runs |
| 6 | re-enter a jail created before the label existed, without `--nix-store` | not recreated; keeps its `…__nix` volume (absent label is read as `project`) |
| 6 | same jail with `--nix-store shared` | logs the mismatch, recreates on `jail-ai-nix`, `/home/agent` preserved |
| 7 | `$JAIL create nixa-host --nix-store host` | `/nix/store` ro + `/nix/var/nix/daemon-socket` mounts, label `host`, `NIX_REMOTE=daemon` and `JAIL_AI_HOST_NIX_BIN` set |
| 7 | `jx nixa-host 'command -v nix; nix --version'` | the **host's** nix, from `/nix/store/…-nix-<host version>/bin` |
| 7 | `jx nixa-host 'nix store info'` | `Store URL: daemon`; note `Trusted:` (see [Security: host mode](#security-host-mode)) |
| 7 | build something in the jail | the path exists on the host afterwards; paths the host already has are not refetched |
| 7 | `jx nixa-host 'touch /nix/store/x'` | `Read-only file system` |
| 7 | `podman exec nixa-host /usr/local/bin/nix-wrapper hello` | enters the devShell and runs it |
| 7 | `env PATH=<podman but no nix> $JAIL create h2 --nix-store host` | fails with "requires the nix client on PATH" |
| 8 | `$JAIL remove nixa -f --volume` | removes `nixa__home`; `jail-ai-nix` still listed by `podman volume ls` |

To confirm `max-jobs` actually parallelises, build several independent derivations at once. Note that
a builder of `/bin/sh -c "sleep 5; …"` does **not** sleep — the sandbox maps only `/bin/sh` (busybox),
so `sleep` is not found and the build finishes instantly. Map it explicitly:

```bash
jx nixa "nix-build /tmp/par.nix --no-out-link \
  --option sandbox-paths '/bin/sh=$BB /bin/sleep=$BB'"   # BB = busybox in the store
```

With 8 × 5s derivations this takes ~5s at `max-jobs = auto` versus ~41s at `--max-jobs 1`.

Cleanup:

```bash
for j in nixa nixb nixa-proj nixa-host plainj; do $JAIL remove $j -f --volume 2>/dev/null; done
rm -rf /tmp/jt    # the shared jail-ai-nix volume is intentionally left in place
```

## Future Enhancements

Potential improvements:
- Nix-specific resource limits
- Binary cache configuration
- Support for `shell.nix` (classic Nix shells)
- Nix channel management

## Notes

- Nix flakes are **experimental** but widely used in the Nix community
- There is no Nix daemon inside the jail; the `agent` user owns the store (or, with
  `--nix-store host`, the host daemon does)
- The Nix store persists in a volume across jail recreations (see *Nix Store*)

## Compatibility

- Works with all existing jail-ai features
- Compatible with resource limits, network isolation, etc.
- Can be combined with other project types (multi-language detection)
- Works with all AI agent integrations
