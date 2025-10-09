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

USER agent

# Install Nix package manager (single-user installation for containers)
# Single-user mode doesn't require a daemon service to be running
RUN curl -L https://nixos.org/nix/install | sh -s -- --no-daemon

# Enable Nix flakes and other experimental features
# Also create wrapper script and setup shell configs in one layer to minimize layer count
RUN mkdir -p /home/agent/.config/nix && \
    echo "experimental-features = nix-command flakes" > /home/agent/.config/nix/nix.conf && \
    echo '#!/usr/bin/env bash' > /home/agent/nix-wrapper && \
    echo '# Nix environment wrapper for jail-ai' >> /home/agent/nix-wrapper && \
    echo '' >> /home/agent/nix-wrapper && \
    echo '# Source Nix environment' >> /home/agent/nix-wrapper && \
    echo 'if [ -e /home/agent/.nix-profile/etc/profile.d/nix.sh ]; then' >> /home/agent/nix-wrapper && \
    echo '  . /home/agent/.nix-profile/etc/profile.d/nix.sh' >> /home/agent/nix-wrapper && \
    echo 'fi' >> /home/agent/nix-wrapper && \
    echo '' >> /home/agent/nix-wrapper && \
    echo '# Ensure Nix paths are in PATH' >> /home/agent/nix-wrapper && \
    echo 'export PATH="${HOME}/.nix-profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"' >> /home/agent/nix-wrapper && \
    echo '' >> /home/agent/nix-wrapper && \
    echo '# If flake.nix exists and we are not already in a nix develop shell, enter it' >> /home/agent/nix-wrapper && \
    echo 'if [ -f /workspace/flake.nix ] && [ -z "$JAIL_AI_NIX_LOADED" ]; then' >> /home/agent/nix-wrapper && \
    echo '  echo "🔵 Nix flake detected, loading development environment..." >&2' >> /home/agent/nix-wrapper && \
    echo '  cd /workspace' >> /home/agent/nix-wrapper && \
    echo '  # Set marker to prevent re-entry and use --command to run inside nix develop' >> /home/agent/nix-wrapper && \
    echo '  export JAIL_AI_NIX_LOADED=1' >> /home/agent/nix-wrapper && \
    echo '  exec nix develop --command "$@"' >> /home/agent/nix-wrapper && \
    echo 'else' >> /home/agent/nix-wrapper && \
    echo '  # No flake or already in nix shell, just execute the command' >> /home/agent/nix-wrapper && \
    echo '  exec "$@"' >> /home/agent/nix-wrapper && \
    echo 'fi' >> /home/agent/nix-wrapper && \
    chmod +x /home/agent/nix-wrapper && \
    echo '' >> /home/agent/.zshrc && \
    echo '# Nix environment' >> /home/agent/.zshrc && \
    echo 'if [ -e /home/agent/.nix-profile/etc/profile.d/nix.sh ]; then' >> /home/agent/.zshrc && \
    echo '  . /home/agent/.nix-profile/etc/profile.d/nix.sh' >> /home/agent/.zshrc && \
    echo 'fi' >> /home/agent/.zshrc && \
    echo '# Ensure Nix paths are in PATH (fallback if sourcing fails)' >> /home/agent/.zshrc && \
    echo 'export PATH="${HOME}/.nix-profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"' >> /home/agent/.zshrc && \
    echo '' >> /home/agent/.zshrc && \
    echo '# Auto-load Nix flake development environment if flake.nix exists in workspace' >> /home/agent/.zshrc && \
    echo '# Only load for interactive shells, not for command execution' >> /home/agent/.zshrc && \
    echo 'if [[ $- == *i* ]] && [ -f /workspace/flake.nix ] && [ -z "$JAIL_AI_NIX_LOADED" ]; then' >> /home/agent/.zshrc && \
    echo '  export JAIL_AI_NIX_LOADED=1' >> /home/agent/.zshrc && \
    echo '  echo "🔵 Nix flake detected in /workspace, loading development environment..."' >> /home/agent/.zshrc && \
    echo '  cd /workspace && exec nix develop' >> /home/agent/.zshrc && \
    echo 'fi' >> /home/agent/.zshrc && \
    echo '' >> /home/agent/.bashrc && \
    echo '# Nix environment' >> /home/agent/.bashrc && \
    echo 'if [ -e /home/agent/.nix-profile/etc/profile.d/nix.sh ]; then' >> /home/agent/.bashrc && \
    echo '  . /home/agent/.nix-profile/etc/profile.d/nix.sh' >> /home/agent/.bashrc && \
    echo 'fi' >> /home/agent/.bashrc && \
    echo '# Ensure Nix paths are in PATH (fallback if sourcing fails)' >> /home/agent/.bashrc && \
    echo 'export PATH="${HOME}/.nix-profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"' >> /home/agent/.bashrc && \
    echo '' >> /home/agent/.bashrc && \
    echo '# Auto-load Nix flake development environment if flake.nix exists in workspace' >> /home/agent/.bashrc && \
    echo '# Only load for interactive shells, not for command execution' >> /home/agent/.bashrc && \
    echo 'if [[ $- == *i* ]] && [ -f /workspace/flake.nix ] && [ -z "$JAIL_AI_NIX_LOADED" ]; then' >> /home/agent/.bashrc && \
    echo '  export JAIL_AI_NIX_LOADED=1' >> /home/agent/.bashrc && \
    echo '  echo "🔵 Nix flake detected in /workspace, loading development environment..."' >> /home/agent/.bashrc && \
    echo '  cd /workspace && exec nix develop' >> /home/agent/.bashrc && \
    echo 'fi' >> /home/agent/.bashrc

WORKDIR /workspace

CMD ["/bin/zsh"]
