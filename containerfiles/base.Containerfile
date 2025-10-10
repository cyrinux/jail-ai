FROM docker.io/library/debian:bookworm-slim

LABEL maintainer="jail-ai"
LABEL description="jail-ai base environment with common tools"

# Build arguments for user/group ID mapping
ARG PUID=1000
ARG PGID=1000

# System-wide paths
ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

# Install base tools and dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Core utilities
    bash \
    zsh \
    coreutils \
    curl \
    wget \
    sl \
    git \
    tig \
    vim \
    nano \
    tree \
    file \
    buildah \
    podman \
    less \
    openssh-client \
    kitty-terminfo \
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
    zip \
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
    fonts-powerline \
    # Terminal info
    ncurses-term \
    # GPG and signing tools
    gnupg \
    gpg-agent \
    pinentry-curses \
    && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI
RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install -y gh \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js LTS (required for all AI agents: Claude, Copilot, Cursor, Gemini, Codex)
RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install yarn and pnpm globally
RUN npm install -g yarn pnpm

# Install Powerlevel10k to system location
RUN git clone --depth=1 https://github.com/romkatv/powerlevel10k.git /usr/share/powerlevel10k

# Set up default shell configs in /etc/skel (will be copied to new users)
# Configure zsh with Powerlevel10k
RUN mkdir -p /etc/skel \
    && echo 'source /usr/share/powerlevel10k/powerlevel10k.zsh-theme' > /etc/skel/.zshrc \
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

# Create a minimal p10k config in /etc/skel with custom jail_agent segment
RUN echo '# Powerlevel10k configuration' > /etc/skel/.p10k.zsh \
    && echo '' >> /etc/skel/.p10k.zsh \
    && echo '# Custom segment for jail-ai agent identification' >> /etc/skel/.p10k.zsh \
    && echo 'function prompt_jail_agent() {' >> /etc/skel/.p10k.zsh \
    && echo '  if [[ -n "${JAIL_AI_AGENT}" ]]; then' >> /etc/skel/.p10k.zsh \
    && echo '    p10k segment -f 15 -b 4 -t "${JAIL_AI_AGENT}"' >> /etc/skel/.p10k.zsh \
    && echo '  fi' >> /etc/skel/.p10k.zsh \
    && echo '}' >> /etc/skel/.p10k.zsh \
    && echo '' >> /etc/skel/.p10k.zsh \
    && echo '# Enable instant prompt (must be near the top)' >> /etc/skel/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_INSTANT_PROMPT=quiet' >> /etc/skel/.p10k.zsh \
    && echo '' >> /etc/skel/.p10k.zsh \
    && echo '# Prompt elements' >> /etc/skel/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_LEFT_PROMPT_ELEMENTS=(jail_agent dir vcs)' >> /etc/skel/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_RIGHT_PROMPT_ELEMENTS=(status command_execution_time background_jobs)' >> /etc/skel/.p10k.zsh \
    && echo '' >> /etc/skel/.p10k.zsh \
    && echo '# Prompt formatting' >> /etc/skel/.p10k.zsh \
    && echo 'typeset -g POWERLEVEL9K_PROMPT_ADD_NEWLINE=true' >> /etc/skel/.p10k.zsh

# Set up bash environment in /etc/skel (for compatibility)
RUN echo 'export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/go/bin:$PATH"' >> /etc/skel/.bashrc \
    && echo '# Custom PS1 with agent identification' >> /etc/skel/.bashrc \
    && echo 'if [ -n "$JAIL_AI_AGENT" ]; then' >> /etc/skel/.bashrc \
    && echo '  export PS1="\[\033[01;34m\]${JAIL_AI_AGENT}\[\033[00m\] \[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "' >> /etc/skel/.bashrc \
    && echo 'else' >> /etc/skel/.bashrc \
    && echo '  export PS1="\[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "' >> /etc/skel/.bashrc \
    && echo 'fi' >> /etc/skel/.bashrc \
    && echo 'alias ll="ls -lah"' >> /etc/skel/.bashrc \
    && echo 'alias rg="rg --color=auto"' >> /etc/skel/.bashrc

# Create agent user with configurable UID/GID
RUN if ! getent group ${PGID} > /dev/null 2>&1; then \
        groupadd -g ${PGID} agent; \
    else \
        GROUP_NAME=$(getent group ${PGID} | cut -d: -f1); \
        if [ "$GROUP_NAME" != "agent" ]; then \
            groupmod -n agent $GROUP_NAME; \
        fi; \
    fi \
    && if ! getent passwd ${PUID} > /dev/null 2>&1; then \
        useradd -m -s /bin/zsh -u ${PUID} -g ${PGID} agent; \
    else \
        USER_NAME=$(getent passwd ${PUID} | cut -d: -f1); \
        if [ "$USER_NAME" != "agent" ]; then \
            usermod -l agent -d /home/agent -m -g ${PGID} -s /bin/zsh $USER_NAME; \
        fi; \
    fi \
    && usermod -aG sudo agent \
    && mkdir -p /etc/sudoers.d \
    && echo 'agent ALL=(ALL) NOPASSWD:ALL' > /etc/sudoers.d/agent \
    && chmod 0440 /etc/sudoers.d/agent

# Create workspace and empty config directories for mounting
RUN mkdir -p /workspace && chown agent:agent /workspace \
    && mkdir -p /home/agent/.claude /home/agent/.config/.copilot /home/agent/.cursor /home/agent/.config/gemini /home/agent/.config/codex /home/agent/.gnupg \
    && chown -R agent:agent /home/agent

# Switch to agent user
USER agent
WORKDIR /workspace

# Set zsh as default shell
ENV SHELL=/bin/zsh

CMD ["/bin/zsh"]
