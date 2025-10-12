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
    ## Network debug
    # iputils-ping \
    # iproute2 \
    # socat \
    # Core utilities
    bash \
    zsh \
    coreutils \
    curl \
    wget \
    git \
    tig \
    vim \
    tree \
    file \
    less \
    openssh-client \
    kitty-terminfo \
    # Containers tooling
    buildah \
    podman \
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
    netcat-openbsd \
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

# Create jail-ai configuration directory and base.zsh script
RUN mkdir -p /usr/local/share/jail-ai && \
    cat > /usr/local/share/jail-ai/base.zsh <<'EOFZSH'
# jail-ai base shell configuration

# # Enable Powerlevel10k instant prompt
# causes weird issue when switching from --no-nix to with nix
# if [[ -r "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh" ]]; then
#   source "${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-${(%):-%n}.zsh"
# fi

# Load Powerlevel10k theme
source /usr/share/powerlevel10k/powerlevel10k.zsh-theme

# Add user-local bin directories to PATH
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/go/bin:$PATH"

# Aliases
alias ll="ls -lah"
alias rg="rg --color=auto"

# FZF integration
source /usr/share/doc/fzf/examples/key-bindings.zsh 2>/dev/null || true
source /usr/share/doc/fzf/examples/completion.zsh 2>/dev/null || true

# History settings
HISTFILE=~/.zsh_history
HISTSIZE=10000
SAVEHIST=10000
setopt SHARE_HISTORY
setopt HIST_IGNORE_ALL_DUPS

# Load Powerlevel10k config if exists
[[ ! -f ~/.p10k.zsh ]] || source ~/.p10k.zsh
EOFZSH

# Set up default shell configs in /etc/skel (will be copied to new users)
RUN mkdir -p /etc/skel && cat > /etc/skel/.zshrc <<'EOFZSHRC'
source /usr/local/share/jail-ai/base.zsh
source /usr/local/share/jail-ai/nix.zsh 2>/dev/null || true

# Auto-load Nix flake development environment if available
# Only for interactive shells, not for command execution
if command -v nix >/dev/null 2>&1 && [[ $- == *i* ]] && [ -f /workspace/flake.nix ] && [ -z "$JAIL_AI_NIX_LOADED" ]; then
  export JAIL_AI_NIX_LOADED=1
  echo "🔵 Nix flake detected in /workspace, loading development environment..."
  cd /workspace
  exec nix develop --command zsh
fi
EOFZSHRC

# Create a minimal p10k config in /etc/skel with custom jail_agent segment
RUN cat > /etc/skel/.p10k.zsh <<'EOFP10K'
# Powerlevel10k configuration

# Custom segment for jail-ai agent identification
function prompt_jail_agent() {
  if [[ -n "${JAIL_AI_AGENT}" ]]; then
    p10k segment -f 15 -b 4 -t "${JAIL_AI_AGENT}"
  fi
}

# Enable instant prompt (must be near the top)
typeset -g POWERLEVEL9K_INSTANT_PROMPT=quiet

# Prompt elements
typeset -g POWERLEVEL9K_LEFT_PROMPT_ELEMENTS=(jail_agent dir vcs)
typeset -g POWERLEVEL9K_RIGHT_PROMPT_ELEMENTS=(status command_execution_time background_jobs)

# Prompt formatting
typeset -g POWERLEVEL9K_PROMPT_ADD_NEWLINE=true
EOFP10K

# Create base.bash script for bash users
RUN cat > /usr/local/share/jail-ai/base.bash <<'EOFBASH'
# jail-ai base bash configuration

# Add user-local bin directories to PATH
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/go/bin:$PATH"

# Custom PS1 with agent identification
if [ -n "$JAIL_AI_AGENT" ]; then
  export PS1="\[\033[01;34m\]${JAIL_AI_AGENT}\[\033[00m\] \[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "
else
  export PS1="\[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "
fi

# Aliases
alias ll="ls -lah"
alias rg="rg --color=auto"
EOFBASH

# Set up bash environment in /etc/skel (for compatibility)
RUN cat > /etc/skel/.bashrc <<'EOFBASHRC'
source /usr/local/share/jail-ai/base.bash
source /usr/local/share/jail-ai/nix.bash 2>/dev/null || true

# Auto-load Nix flake development environment if available
# Only for interactive shells, not for command execution
if command -v nix >/dev/null 2>&1 && [[ $- == *i* ]] && [ -f /workspace/flake.nix ] && [ -z "$JAIL_AI_NIX_LOADED" ]; then
  export JAIL_AI_NIX_LOADED=1
  echo "🔵 Nix flake detected in /workspace, loading development environment..."
  cd /workspace
  exec nix develop --command zsh
fi
EOFBASHRC

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
    && mkdir -p /home/agent/.claude /home/agent/.config/.copilot /home/agent/.cursor /home/agent/.gemini /home/agent/.config/codex /home/agent/.gnupg \
    && chown -R agent:agent /home/agent

# Switch to agent user
USER agent
WORKDIR /workspace

# Set zsh as default shell
ENV SHELL=/bin/zsh

CMD ["/bin/zsh"]
