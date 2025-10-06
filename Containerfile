FROM docker.io/library/debian:bookworm-slim

LABEL maintainer="jail-ai"
LABEL description="AI Agent development environment with common tools"

# Build arguments for user/group ID mapping
ARG PUID=1000
ARG PGID=1000

# System-wide installation paths for development tools
# Note: CARGO_HOME, GOPATH, etc. are NOT set here - they default to user home directories
# so that caches/dependencies are writable and persist in /home/agent volume
ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8 \
    RUSTUP_HOME="/usr/local/rustup" \
    PATH="/usr/local/cargo/bin:/usr/local/poetry/bin:/usr/local/go/bin:${PATH}"

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
    # System libraries for development
    libclang-dev \
    libdbus-1-dev \
    libglib2.0-dev \
    libudev-dev \
    libxkbcommon-dev \
    libinput-dev \
    libpulse-dev \
    libvulkan-dev \
    vulkan-tools \
    libv4l-dev \
    v4l-utils \
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
    # Terminal info
    ncurses-term \
    kitty-terminfo \
    foot-terminfo \
    # GPG and signing tools
    gnupg \
    gpg-agent \
    pinentry-curses \
    && rm -rf /var/lib/apt/lists/*

# Install Go to /usr/local
ARG GO_VERSION=1.23.4
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://go.dev/dl/go${GO_VERSION}.linux-${ARCH}.tar.gz" | tar -C /usr/local -xz \
    && ln -s /usr/local/go/bin/go /usr/local/bin/go \
    && ln -s /usr/local/go/bin/gofmt /usr/local/bin/gofmt

# Install Node.js and npm (LTS version)
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install yarn and pnpm globally
RUN npm install -g yarn pnpm

# Install GitHub CLI (gh)
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install -y gh \
    && rm -rf /var/lib/apt/lists/*

# Install Python, Java, and related tools
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-venv \
    default-jdk \
    && rm -rf /var/lib/apt/lists/* \
    && ln -s /usr/bin/python3 /usr/bin/python

# Install common Python development tools
RUN pip3 install --no-cache-dir --break-system-packages \
    black \
    pylint \
    mypy \
    pytest

# Install Rust and Cargo binaries to system location (/usr/local/cargo/bin)
# CARGO_HOME is set during installation only, not at runtime
# At runtime, CARGO_HOME defaults to ~/.cargo for user-writable caches/deps
RUN export CARGO_HOME=/usr/local/cargo \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
    && $CARGO_HOME/bin/rustup default stable \
    && $CARGO_HOME/bin/rustup component add clippy rustfmt

# Install Poetry binary to system location (/usr/local/poetry/bin)
# POETRY_HOME is set during installation only
# At runtime, poetry cache defaults to ~/.cache/pypoetry (user-writable)
RUN export POETRY_HOME=/usr/local/poetry \
    && curl -sSL https://install.python-poetry.org | python3 -

# Install AI coding assistants globally
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

# Install Cursor Agent CLI to system location
# Cursor installer creates versioned dir in ~/.local/share/cursor-agent and symlink in ~/.local/bin
# Move entire installation to /usr/local to avoid /root pollution
RUN curl -fsSL https://cursor.com/install | bash \
    && if [ -d /root/.local/share/cursor-agent ]; then \
        mkdir -p /usr/local/share && \
        mv /root/.local/share/cursor-agent /usr/local/share/cursor-agent && \
        CURSOR_BIN=$(find /usr/local/share/cursor-agent/versions -name cursor-agent -type f | head -n1) && \
        if [ -n "$CURSOR_BIN" ]; then \
            ln -s "$CURSOR_BIN" /usr/local/bin/cursor-agent; \
        fi; \
    fi

# Install Powerlevel10k to system location
RUN git clone --depth=1 https://github.com/romkatv/powerlevel10k.git /usr/share/powerlevel10k

# Set up default shell configs in /etc/skel (will be copied to new users)
# Configure zsh with Powerlevel10k
RUN echo 'source /usr/share/powerlevel10k/powerlevel10k.zsh-theme' > /etc/skel/.zshrc \
    && echo '# Enable Powerlevel10k instant prompt' >> /etc/skel/.zshrc \
    && echo 'if [[ -r "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh" ]]; then' >> /etc/skel/.zshrc \
    && echo '  source "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh"' >> /etc/skel/.zshrc \
    && echo 'fi' >> /etc/skel/.zshrc \
    && echo '' >> /etc/skel/.zshrc \
    && echo '# Add user-local bin directories to PATH' >> /etc/skel/.zshrc \
    && echo 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/go/bin:$PATH"' >> /etc/skel/.zshrc \
    && echo '' >> /etc/skel/.zshrc \
    && echo '# Aliases' >> /etc/skel/.zshrc \
    && echo 'alias ll="ls -lah"' >> /etc/skel/.zshrc \
    && echo 'alias rg="rg --color=auto"' >> /etc/skel/.zshrc \
    && echo '' >> /etc/skel/.zshrc \
    && echo '# FZF integration' >> /etc/skel/.zshrc \
    && echo 'source /usr/share/doc/fzf/examples/key-bindings.zsh 2>/dev/null || true' >> /etc/skel/.zshrc \
    && echo 'source /usr/share/doc/fzf/examples/completion.zsh 2>/dev/null || true' >> /etc/skel/.zshrc \
    && echo '' >> /etc/skel/.zshrc \
    && echo '# History settings' >> /etc/skel/.zshrc \
    && echo 'HISTFILE=~/.zsh_history' >> /etc/skel/.zshrc \
    && echo 'HISTSIZE=10000' >> /etc/skel/.zshrc \
    && echo 'SAVEHIST=10000' >> /etc/skel/.zshrc \
    && echo 'setopt SHARE_HISTORY' >> /etc/skel/.zshrc \
    && echo 'setopt HIST_IGNORE_ALL_DUPS' >> /etc/skel/.zshrc \
    && echo '' >> /etc/skel/.zshrc \
    && echo '# Load Powerlevel10k config if exists' >> /etc/skel/.zshrc \
    && echo '[[ ! -f ~/.p10k.zsh ]] || source ~/.p10k.zsh' >> /etc/skel/.zshrc

# Create a minimal p10k config in /etc/skel
RUN echo '# Powerlevel10k configuration' > /etc/skel/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_LEFT_PROMPT_ELEMENTS=(dir vcs)' >> /etc/skel/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_RIGHT_PROMPT_ELEMENTS=(status command_execution_time background_jobs)' >> /etc/skel/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_PROMPT_ADD_NEWLINE=true' >> /etc/skel/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_INSTANT_PROMPT=quiet' >> /etc/skel/.p10k.zsh

# Set up bash environment in /etc/skel (for compatibility)
RUN echo 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/go/bin:$PATH"' >> /etc/skel/.bashrc \
    && echo 'export PS1="\[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "' >> /etc/skel/.bashrc \
    && echo 'alias ll="ls -lah"' >> /etc/skel/.bashrc \
    && echo 'alias rg="rg --color=auto"' >> /etc/skel/.bashrc

# Create agent user with configurable UID/GID
# /etc/skel contents will be automatically copied to /home/agent
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

# Create workspace and empty config directories for mounting
# These directories will be empty in the image, ready for bind mounts
RUN mkdir -p /workspace && chown agent:agent /workspace \
    && mkdir -p /home/agent/.claude /home/agent/.config/.copilot /home/agent/.cursor /home/agent/.gnupg \
    && chown -R agent:agent /home/agent

# Switch to agent user
USER agent
WORKDIR /workspace

# Set zsh as default shell
ENV SHELL=/usr/bin/zsh

CMD ["/usr/bin/zsh"]
