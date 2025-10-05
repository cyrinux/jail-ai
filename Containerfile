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

# Create workspace directory
RUN mkdir -p /workspace
WORKDIR /workspace

# Set up shell environment
RUN echo 'export PS1="\[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "' >> /root/.bashrc \
    && echo 'alias ll="ls -lah"' >> /root/.bashrc \
    && echo 'alias rg="rg --color=auto"' >> /root/.bashrc

CMD ["/bin/bash"]
