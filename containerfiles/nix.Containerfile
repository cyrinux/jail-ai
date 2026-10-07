ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Nix development environment with flakes support"

USER root

# Install Nix dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Create /nix directory with proper permissions for single-user install
RUN mkdir -p /nix && chown -R agent:agent /nix

# Create directory to store nix-profile with proper permissions for single-user install
RUN mkdir -p /usr/local/nix-state && chown -R agent:agent /usr/local/nix-state

# Create directory for the Nix store seed (restored into /nix when a volume lacks this image's Nix)
RUN mkdir -p /usr/local/nix-seed && chown -R agent:agent /usr/local/nix-seed

# Create Nix wrapper script in /usr/local/bin (as root)
RUN cat > /usr/local/bin/nix-wrapper <<'EOFWRAPPER' && chmod +x /usr/local/bin/nix-wrapper
#!/usr/bin/env bash
# Nix environment wrapper for jail-ai

# Source Nix environment
if [ -e /usr/local/nix-state/nix/profiles/profile/etc/profile.d/nix.sh ]; then
  . /usr/local/nix-state/nix/profiles/profile/etc/profile.d/nix.sh
fi

# Ensure Nix paths are in PATH
export PATH="/usr/local/nix-state/nix/profiles/profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"

# Make sure /nix provides a working Nix (shared/old volumes, host store)
. /usr/local/share/jail-ai/nix-init.sh

# If flake.nix exists and we are not already in a nix develop shell, enter it
if [ -f /workspace/flake.nix ] && [ -z "$JAIL_AI_NIX_LOADED" ]; then
  echo "🔵 Nix flake detected, loading development environment..." >&2
  cd /workspace
  # Set marker to prevent re-entry and use --command to run inside nix develop
  export JAIL_AI_NIX_LOADED=1
  exec nix develop --command "$@"
else
  # No flake or already in nix shell, just execute the command
  exec "$@"
fi
EOFWRAPPER

# Enable Nix flakes and parallel builds (Nix defaults to max-jobs = 1; the installer sets nothing)
RUN mkdir -p /etc/nix && \
    printf '%s\n' "experimental-features = nix-command flakes" "max-jobs = auto" > /etc/nix/nix.conf

# Seed restore script: copies this image's Nix closure into /nix/store when missing
# (a shared or pre-existing /nix volume is not refreshed by podman on image upgrades)
# and pins it with a GC root so a garbage collection in another jail cannot remove it
RUN cat > /usr/local/bin/jail-ai-nix-seed <<'EOFSEED' && chmod +x /usr/local/bin/jail-ai-nix-seed
#!/usr/bin/env bash
set -euo pipefail

SEED=/usr/local/nix-seed
PROFILE=$(cat "$SEED/profile")

mkdir -p /nix/store /nix/var/nix/gcroots/jail-ai
exec 9>/nix/var/jail-ai-seed.lock
flock 9

if [ ! -x "$PROFILE/bin/nix" ]; then
  echo "🔵 Restoring Nix into /nix/store..." >&2
  for path in "$SEED"/store/*; do
    [ -e "/nix/store/${path##*/}" ] || cp -a "$path" /nix/store/
  done
  "$PROFILE/bin/nix-store" --load-db < "$SEED/reginfo"
fi

ln -sfn "$PROFILE" "/nix/var/nix/gcroots/jail-ai/${PROFILE##*/}"
EOFSEED

# Nix init, sourced by nix-wrapper, zsh and bash (POSIX sh, cheap on the fast path)
RUN cat > /usr/local/share/jail-ai/nix-init.sh <<'EOFINIT'
# jail-ai nix store init
if [ -n "${JAIL_AI_HOST_NIX_BIN:-}" ]; then
  # --nix-store host: the host store is mounted read-only, use the host's nix client
  case ":$PATH:" in
    *":$JAIL_AI_HOST_NIX_BIN:"*) ;;
    *) export PATH="$JAIL_AI_HOST_NIX_BIN:$PATH" ;;
  esac
elif [ -r /usr/local/nix-seed/profile ]; then
  read -r _jail_ai_nix_profile < /usr/local/nix-seed/profile
  if [ ! -x "$_jail_ai_nix_profile/bin/nix" ] || \
     [ ! -L "/nix/var/nix/gcroots/jail-ai/${_jail_ai_nix_profile##*/}" ]; then
    /usr/local/bin/jail-ai-nix-seed || echo "⚠️  jail-ai: failed to restore Nix into /nix/store" >&2
  fi
  unset _jail_ai_nix_profile
fi
EOFINIT

# Create nix.zsh configuration script
RUN cat > /usr/local/share/jail-ai/nix.zsh <<'EOFZSH'
# jail-ai nix shell configuration

# Source Nix environment
if [ -e /usr/local/nix-state/nix/profiles/profile/etc/profile.d/nix.sh ]; then
  . /usr/local/nix-state/nix/profiles/profile/etc/profile.d/nix.sh
fi

# Ensure Nix paths are in PATH (fallback if sourcing fails)
export PATH="/usr/local/nix-state/nix/profiles/profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"

# Make sure /nix provides a working Nix (shared/old volumes, host store)
. /usr/local/share/jail-ai/nix-init.sh
EOFZSH

# Create nix.bash configuration script
RUN cat > /usr/local/share/jail-ai/nix.bash <<'EOFBASH'
# jail-ai nix bash configuration

# Source Nix environment
if [ -e /usr/local/nix-state/nix/profiles/profile/etc/profile.d/nix.sh ]; then
  . /usr/local/nix-state/nix/profiles/profile/etc/profile.d/nix.sh
fi

# Ensure Nix paths are in PATH (fallback if sourcing fails)
export PATH="/usr/local/nix-state/nix/profiles/profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"

# Make sure /nix provides a working Nix (shared/old volumes, host store)
. /usr/local/share/jail-ai/nix-init.sh
EOFBASH

USER agent

# Install Nix package manager (single-user installation for containers)
# Single-user mode doesn't require a daemon service to be running
RUN curl -L https://nixos.org/nix/install | env XDG_STATE_HOME=/usr/local/nix-state sh -s -- --no-daemon --no-modify-profile

# Snapshot the installed Nix closure and its DB registration as the store seed
RUN PROFILE=$(readlink -f /usr/local/nix-state/nix/profiles/profile) && \
    CLOSURE=$("$PROFILE/bin/nix-store" -qR "$PROFILE") && \
    mkdir -p /usr/local/nix-seed/store && \
    cp -a $CLOSURE /usr/local/nix-seed/store/ && \
    "$PROFILE/bin/nix-store" --dump-db $CLOSURE > /usr/local/nix-seed/reginfo && \
    echo "$PROFILE" > /usr/local/nix-seed/profile && \
    /usr/local/bin/jail-ai-nix-seed

WORKDIR /workspace

CMD ["/bin/zsh"]
