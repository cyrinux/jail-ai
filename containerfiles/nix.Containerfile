ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Nix development environment with flakes support"

USER root

# Install Nix dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Install Nix package manager (multi-user installation)
RUN curl -L https://nixos.org/nix/install | sh -s -- --daemon --yes

# Enable Nix flakes and other experimental features
RUN mkdir -p /etc/nix && \
    echo "experimental-features = nix-command flakes" >> /etc/nix/nix.conf

# Add Nix to PATH for all users
ENV PATH="/nix/var/nix/profiles/default/bin:${PATH}"

# Create necessary directories for Nix
RUN mkdir -p /nix/var/nix/profiles/per-user/agent && \
    chown -R agent:agent /nix/var/nix/profiles/per-user/agent

USER agent
WORKDIR /workspace

# Configure Nix for the agent user
RUN mkdir -p /home/agent/.config/nix && \
    echo "experimental-features = nix-command flakes" > /home/agent/.config/nix/nix.conf

# Add Nix environment setup to shell configs
RUN echo '' >> /home/agent/.zshrc && \
    echo '# Nix environment' >> /home/agent/.zshrc && \
    echo 'if [ -e /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh ]; then' >> /home/agent/.zshrc && \
    echo '  . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh' >> /home/agent/.zshrc && \
    echo 'fi' >> /home/agent/.zshrc && \
    echo '' >> /home/agent/.bashrc && \
    echo '# Nix environment' >> /home/agent/.bashrc && \
    echo 'if [ -e /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh ]; then' >> /home/agent/.bashrc && \
    echo '  . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh' >> /home/agent/.bashrc && \
    echo 'fi'

CMD ["/bin/zsh"]
