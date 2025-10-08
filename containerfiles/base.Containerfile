FROM docker.io/library/alpine:3.20

LABEL maintainer="jail-ai"
LABEL description="jail-ai base environment with common tools"

# Build arguments for user/group ID mapping
ARG PUID=1000
ARG PGID=1000

# System-wide paths
ENV LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

# Install base tools and dependencies
# Alpine uses apk instead of apt-get
RUN apk add --no-cache \
    # Core utilities
    bash \
    zsh \
    coreutils \
    curl \
    wget \
    git \
    tig \
    vim \
    nano \
    tree \
    file \
    less \
    # Build essentials
    build-base \
    pkgconf \
    openssl-dev \
    # Search and text processing
    ripgrep \
    fd \
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
    ncurses-terminfo-base \
    # GPG and signing tools
    gnupg \
    pinentry

# Install GitHub CLI
RUN apk add --no-cache github-cli

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
    && echo 'source /usr/share/fzf/key-bindings.zsh 2>/dev/null || true' >> /etc/skel/.zshrc \
    && echo 'source /usr/share/fzf/completion.zsh 2>/dev/null || true' >> /etc/skel/.zshrc \
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
RUN addgroup -g ${PGID} agent 2>/dev/null || true \
    && adduser -D -s /bin/zsh -u ${PUID} -G agent agent \
    && echo "agent ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/agent \
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
