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

# Enable Nix flakes and other experimental features
RUN mkdir -p /etc/nix && \
    echo "experimental-features = nix-command flakes" > /etc/nix/nix.conf

# Create nix.zsh configuration script
RUN cat > /usr/local/share/jail-ai/nix.zsh <<'EOFZSH'
# jail-ai nix shell configuration

# Source Nix environment
if [ -e /usr/local/nix-state/nix/profiles/profile/etc/profile.d/nix.sh ]; then
  . /usr/local/nix-state/nix/profiles/profile/etc/profile.d/nix.sh
fi

# Ensure Nix paths are in PATH (fallback if sourcing fails)
export PATH="/usr/local/nix-state/nix/profiles/profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"
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
EOFBASH

USER agent

# Install Nix package manager (single-user installation for containers)
# Single-user mode doesn't require a daemon service to be running
RUN curl -L https://nixos.org/nix/install | env XDG_STATE_HOME=/usr/local/nix-state sh -s -- --no-daemon --no-modify-profile

WORKDIR /workspace

CMD ["/bin/zsh"]
