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
RUN mkdir -p /home/agent/.config/nix && \
    echo "experimental-features = nix-command flakes" > /home/agent/.config/nix/nix.conf

# Add Nix environment setup to shell configs
# Source the nix.sh profile script which sets up NIX_PROFILES, PATH, and other environment variables
RUN echo '' >> /home/agent/.zshrc && \
    echo '# Nix environment' >> /home/agent/.zshrc && \
    echo 'if [ -e /home/agent/.nix-profile/etc/profile.d/nix.sh ]; then' >> /home/agent/.zshrc && \
    echo '  . /home/agent/.nix-profile/etc/profile.d/nix.sh' >> /home/agent/.zshrc && \
    echo 'fi' >> /home/agent/.zshrc && \
    echo '# Ensure Nix paths are in PATH (fallback if sourcing fails)' >> /home/agent/.zshrc && \
    echo 'export PATH="${HOME}/.nix-profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"' >> /home/agent/.zshrc && \
    echo '' >> /home/agent/.bashrc && \
    echo '# Nix environment' >> /home/agent/.bashrc && \
    echo 'if [ -e /home/agent/.nix-profile/etc/profile.d/nix.sh ]; then' >> /home/agent/.bashrc && \
    echo '  . /home/agent/.nix-profile/etc/profile.d/nix.sh' >> /home/agent/.bashrc && \
    echo 'fi' >> /home/agent/.bashrc && \
    echo '# Ensure Nix paths are in PATH (fallback if sourcing fails)' >> /home/agent/.bashrc && \
    echo 'export PATH="${HOME}/.nix-profile/bin:/nix/var/nix/profiles/default/bin:${PATH}"' >> /home/agent/.bashrc

WORKDIR /workspace

CMD ["/bin/zsh"]
