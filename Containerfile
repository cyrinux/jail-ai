FROM docker.io/library/debian:bookworm-slim

LABEL maintainer="jail-ai"
LABEL description="AI Agent development environment with common tools"

# Build arguments for user/group ID mapping
ARG PUID=1000
ARG PGID=1000

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    CARGO_HOME="/home/agent/.cargo" \
    RUSTUP_HOME="/home/agent/.rustup" \
    GOPATH="/home/agent/go" \
    PATH="/home/agent/.local/bin:/home/agent/.cargo/bin:/home/agent/go/bin:/usr/local/go/bin:${PATH}"

# Install base tools and dependencies
RUN apt-get update && apt-get install -y \
    # Core utilities
    bash \
    coreutils \
    curl \
    wget \
    git \
    vim \
    nano \
    tree \
    file \
    less \
    # Build essentials
    build-essential \
    pkg-config \
    libssl-dev \
    # Search and text processing
    ripgrep \
    fd-find \
    jq \
    fzf \
    # Archive tools
    tar \
    gzip \
    unzip \
    # Network tools
    ca-certificates \
    # Process management
    procps \
    htop \
    # Additional utilities
    tmux \
    screen \
    sudo \
    # Shell enhancements
    zsh \
    fonts-powerline \
    && rm -rf /var/lib/apt/lists/*

# Create agent user with configurable UID/GID
# Handle case where GID/UID already exist by checking first
RUN if ! getent group ${PGID} > /dev/null 2>&1; then \
        groupadd -g ${PGID} agent; \
    else \
        GROUP_NAME=$(getent group ${PGID} | cut -d: -f1); \
        if [ "$GROUP_NAME" != "agent" ]; then \
            groupmod -n agent $GROUP_NAME; \
        fi; \
    fi \
    && if ! getent passwd ${PUID} > /dev/null 2>&1; then \
        useradd -m -s /usr/bin/zsh -u ${PUID} -g ${PGID} agent; \
    else \
        USER_NAME=$(getent passwd ${PUID} | cut -d: -f1); \
        if [ "$USER_NAME" != "agent" ]; then \
            usermod -l agent -d /home/agent -m -g ${PGID} -s /usr/bin/zsh $USER_NAME; \
        fi; \
    fi \
    && usermod -aG sudo agent \
    && mkdir -p /etc/sudoers.d \
    && echo 'agent ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/agent \
    && chmod 0440 /etc/sudoers.d/agent

# Install Go (requires root for /usr/local)
ARG GO_VERSION=1.23.4
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://go.dev/dl/go${GO_VERSION}.linux-${ARCH}.tar.gz" | tar -C /usr/local -xz \
    && ln -s /usr/local/go/bin/go /usr/local/bin/go \
    && ln -s /usr/local/go/bin/gofmt /usr/local/bin/gofmt

# Install Node.js and npm (LTS version)
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI (gh)
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install -y gh \
    && rm -rf /var/lib/apt/lists/*

# Switch to agent user for Rust installation
USER agent
WORKDIR /home/agent

# Install Rust and Cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && . "$HOME/.cargo/env" \
    && rustup default stable \
    && rustup component add clippy rustfmt

# Switch back to root for system-wide installations
USER root

# Install Python and pip
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/* \
    && ln -s /usr/bin/python3 /usr/bin/python

# Install common development CLI tools
RUN pip3 install --no-cache-dir --break-system-packages \
    black \
    pylint \
    mypy \
    pytest

# Install AI coding assistants (Claude Code and GitHub Copilot as root)
RUN npm install -g @anthropic-ai/claude-code \
    && npm install -g @github/copilot

# Create Claude wrapper with --dangerously-skip-permissions
RUN CLAUDE_BIN=$(which claude || echo "") && \
    if [ -n "$CLAUDE_BIN" ]; then \
        mv "$CLAUDE_BIN" "${CLAUDE_BIN}-bin" && \
        echo '#!/bin/bash' > "$CLAUDE_BIN" && \
        echo "exec ${CLAUDE_BIN}-bin --dangerously-skip-permissions \"\$@\"" >> "$CLAUDE_BIN" && \
        chmod +x "$CLAUDE_BIN"; \
    fi

# Switch back to agent user
USER agent
WORKDIR /home/agent

# Install Cursor Agent CLI (as agent user so it installs to ~/.local/bin)
RUN mkdir -p /home/agent/.local/bin \
    && curl https://cursor.com/install -fsSL | bash \
    && echo 'Cursor Agent CLI installed to ~/.local/bin/cursor-agent'

# Install and configure Powerlevel10k for zsh
RUN git clone --depth=1 https://github.com/romkatv/powerlevel10k.git /home/agent/.powerlevel10k

# Configure zsh with Powerlevel10k
RUN echo 'source /home/agent/.powerlevel10k/powerlevel10k.zsh-theme' > /home/agent/.zshrc \
    && echo '# Enable Powerlevel10k instant prompt' >> /home/agent/.zshrc \
    && echo 'if [[ -r "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh" ]]; then' >> /home/agent/.zshrc \
    && echo '  source "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh"' >> /home/agent/.zshrc \
    && echo 'fi' >> /home/agent/.zshrc \
    && echo '' >> /home/agent/.zshrc \
    && echo '# Aliases' >> /home/agent/.zshrc \
    && echo 'alias ll="ls -lah"' >> /home/agent/.zshrc \
    && echo 'alias rg="rg --color=auto"' >> /home/agent/.zshrc \
    && echo '' >> /home/agent/.zshrc \
    && echo '# FZF integration' >> /home/agent/.zshrc \
    && echo 'source /usr/share/doc/fzf/examples/key-bindings.zsh 2>/dev/null || true' >> /home/agent/.zshrc \
    && echo 'source /usr/share/doc/fzf/examples/completion.zsh 2>/dev/null || true' >> /home/agent/.zshrc \
    && echo '' >> /home/agent/.zshrc \
    && echo '# History settings' >> /home/agent/.zshrc \
    && echo 'HISTFILE=~/.zsh_history' >> /home/agent/.zshrc \
    && echo 'HISTSIZE=10000' >> /home/agent/.zshrc \
    && echo 'SAVEHIST=10000' >> /home/agent/.zshrc \
    && echo 'setopt SHARE_HISTORY' >> /home/agent/.zshrc \
    && echo 'setopt HIST_IGNORE_ALL_DUPS' >> /home/agent/.zshrc \
    && echo '' >> /home/agent/.zshrc \
    && echo '# Load Powerlevel10k config if exists' >> /home/agent/.zshrc \
    && echo '[[ ! -f ~/.p10k.zsh ]] || source ~/.p10k.zsh' >> /home/agent/.zshrc

# Create a minimal p10k config
RUN echo '# Powerlevel10k configuration' > /home/agent/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_LEFT_PROMPT_ELEMENTS=(dir vcs)' >> /home/agent/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_RIGHT_PROMPT_ELEMENTS=(status command_execution_time background_jobs)' >> /home/agent/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_PROMPT_ADD_NEWLINE=true' >> /home/agent/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_INSTANT_PROMPT=quiet' >> /home/agent/.p10k.zsh

# Set up bash environment (for compatibility)
RUN echo 'export PS1="\[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "' >> /home/agent/.bashrc \
    && echo 'alias ll="ls -lah"' >> /home/agent/.bashrc \
    && echo 'alias rg="rg --color=auto"' >> /home/agent/.bashrc

# Create workspace directory as root and set ownership
USER root
RUN mkdir -p /workspace && chown agent:agent /workspace

# Switch back to agent user
USER agent
WORKDIR /workspace

# Set zsh as default shell
ENV SHELL=/usr/bin/zsh

CMD ["/usr/bin/zsh"]
