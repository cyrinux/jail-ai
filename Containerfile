FROM docker.io/library/debian:bookworm-slim

LABEL maintainer="jail-ai"
LABEL description="AI Agent development environment with common tools"

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    CARGO_HOME="/root/.cargo" \
    RUSTUP_HOME="/root/.rustup" \
    GOPATH="/root/go" \
    PATH="/root/.local/bin:/root/.cargo/bin:/root/go/bin:/usr/local/go/bin:${PATH}"

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
    # Shell enhancements
    zsh \
    fonts-powerline \
    && rm -rf /var/lib/apt/lists/*

# Install Rust and Cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && . "$HOME/.cargo/env" \
    && rustup default stable \
    && rustup component add clippy rustfmt

# Install Go
ARG GO_VERSION=1.23.4
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://go.dev/dl/go${GO_VERSION}.linux-${ARCH}.tar.gz" | tar -C /usr/local -xz \
    && ln -s /usr/local/go/bin/go /usr/local/bin/go \
    && ln -s /usr/local/go/bin/gofmt /usr/local/bin/gofmt

# Install Node.js and npm (LTS version)
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

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

# Install AI coding assistants
RUN npm install -g @anthropic-ai/claude-code \
    && npm install -g @github/copilot \
    && curl https://cursor.com/install -fsSL | bash

# Install and configure Powerlevel10k for zsh
RUN git clone --depth=1 https://github.com/romkatv/powerlevel10k.git /root/.powerlevel10k

# Configure zsh with Powerlevel10k
RUN echo 'source /root/.powerlevel10k/powerlevel10k.zsh-theme' > /root/.zshrc \
    && echo '# Enable Powerlevel10k instant prompt' >> /root/.zshrc \
    && echo 'if [[ -r "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh" ]]; then' >> /root/.zshrc \
    && echo '  source "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh"' >> /root/.zshrc \
    && echo 'fi' >> /root/.zshrc \
    && echo '' >> /root/.zshrc \
    && echo '# Aliases' >> /root/.zshrc \
    && echo 'alias ll="ls -lah"' >> /root/.zshrc \
    && echo 'alias rg="rg --color=auto"' >> /root/.zshrc \
    && echo '' >> /root/.zshrc \
    && echo '# FZF integration' >> /root/.zshrc \
    && echo 'source /usr/share/doc/fzf/examples/key-bindings.zsh 2>/dev/null || true' >> /root/.zshrc \
    && echo 'source /usr/share/doc/fzf/examples/completion.zsh 2>/dev/null || true' >> /root/.zshrc \
    && echo '' >> /root/.zshrc \
    && echo '# History settings' >> /root/.zshrc \
    && echo 'HISTFILE=~/.zsh_history' >> /root/.zshrc \
    && echo 'HISTSIZE=10000' >> /root/.zshrc \
    && echo 'SAVEHIST=10000' >> /root/.zshrc \
    && echo 'setopt SHARE_HISTORY' >> /root/.zshrc \
    && echo 'setopt HIST_IGNORE_ALL_DUPS' >> /root/.zshrc \
    && echo '' >> /root/.zshrc \
    && echo '# Load Powerlevel10k config if exists' >> /root/.zshrc \
    && echo '[[ ! -f ~/.p10k.zsh ]] || source ~/.p10k.zsh' >> /root/.zshrc

# Create a minimal p10k config
RUN echo '# Powerlevel10k configuration' > /root/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_LEFT_PROMPT_ELEMENTS=(dir vcs)' >> /root/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_RIGHT_PROMPT_ELEMENTS=(status command_execution_time background_jobs)' >> /root/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_PROMPT_ADD_NEWLINE=true' >> /root/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_INSTANT_PROMPT=quiet' >> /root/.p10k.zsh

# Create workspace directory
RUN mkdir -p /workspace
WORKDIR /workspace

# Set up bash environment (for compatibility)
RUN echo 'export PS1="\[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "' >> /root/.bashrc \
    && echo 'alias ll="ls -lah"' >> /root/.bashrc \
    && echo 'alias rg="rg --color=auto"' >> /root/.bashrc

# Set zsh as default shell
ENV SHELL=/usr/bin/zsh

CMD ["/usr/bin/zsh"]
