FROM docker.io/library/debian:bookworm-slim

LABEL maintainer="jail-ai"
LABEL description="jail-ai base environment with common tools (macOS/apple-container)"

ARG PUID=1000
ARG PGID=1000

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

RUN apt-get update && apt-get install -y --no-install-recommends \
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
    build-essential \
    pkg-config \
    libssl-dev \
    ripgrep \
    fd-find \
    jq \
    fzf \
    tar \
    gzip \
    unzip \
    zip \
    ca-certificates \
    netcat-openbsd \
    procps \
    htop \
    tini \
    tmux \
    screen \
    sudo \
    fonts-powerline \
    ncurses-term \
    gnupg \
    gpg-agent \
    pinentry-curses \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg \
    && chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null \
    && apt-get update \
    && apt-get install -y gh \
    && rm -rf /var/lib/apt/lists/*

RUN curl -fsSL https://deb.nodesource.com/setup_lts.x | bash - \
    && apt-get install -y --no-install-recommends nodejs \
    && rm -rf /var/lib/apt/lists/*

RUN npm install -g yarn pnpm

RUN git clone --depth=1 https://github.com/romkatv/powerlevel10k.git /usr/share/powerlevel10k

RUN mkdir -p /usr/local/share/jail-ai && \
    cat > /usr/local/share/jail-ai/base.zsh <<'EOFZSH'
source /usr/share/powerlevel10k/powerlevel10k.zsh-theme
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/go/bin:$PATH"
alias ll="ls -lah"
alias rg="rg --color=auto"
source /usr/share/doc/fzf/examples/key-bindings.zsh 2>/dev/null || true
source /usr/share/doc/fzf/examples/completion.zsh 2>/dev/null || true
HISTFILE=~/.zsh_history
HISTSIZE=10000
SAVEHIST=10000
setopt SHARE_HISTORY
setopt HIST_IGNORE_ALL_DUPS
[[ ! -f ~/.p10k.zsh ]] || source ~/.p10k.zsh
EOFZSH

RUN mkdir -p /etc/skel && cat > /etc/skel/.zshrc <<'EOFZSHRC'
source /usr/local/share/jail-ai/base.zsh
source /usr/local/share/jail-ai/nix.zsh 2>/dev/null || true

if command -v nix >/dev/null 2>&1 && [[ $- == *i* ]] && [ -f /workspace/flake.nix ] && [ -z "$JAIL_AI_NIX_LOADED" ]; then
  export JAIL_AI_NIX_LOADED=1
  echo "🔵 Nix flake detected in /workspace, loading development environment..."
  cd /workspace
  exec nix develop --command zsh
fi
EOFZSHRC

RUN cat > /etc/skel/.p10k.zsh <<'EOFP10K'
function prompt_jail_agent() {
  if [[ -n "${JAIL_AI_AGENT}" ]]; then
    p10k segment -f 15 -b 4 -t "${JAIL_AI_AGENT}"
  fi
}
typeset -g POWERLEVEL9K_INSTANT_PROMPT=quiet
typeset -g POWERLEVEL9K_LEFT_PROMPT_ELEMENTS=(jail_agent dir vcs)
typeset -g POWERLEVEL9K_RIGHT_PROMPT_ELEMENTS=(status command_execution_time background_jobs)
typeset -g POWERLEVEL9K_PROMPT_ADD_NEWLINE=true
EOFP10K

RUN cat > /usr/local/share/jail-ai/base.bash <<'EOFBASH'
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/go/bin:$PATH"
if [ -n "$JAIL_AI_AGENT" ]; then
  export PS1="\[\033[01;34m\]${JAIL_AI_AGENT}\[\033[00m\] \[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "
else
  export PS1="\[\033[01;32m\]jail-ai\[\033[00m\]:\[\033[01;34m\]\w\[\033[00m\]\$ "
fi
alias ll="ls -lah"
alias rg="rg --color=auto"
EOFBASH

RUN cat > /etc/skel/.bashrc <<'EOFBASHRC'
source /usr/local/share/jail-ai/base.bash
source /usr/local/share/jail-ai/nix.bash 2>/dev/null || true

if command -v nix >/dev/null 2>&1 && [[ $- == *i* ]] && [ -f /workspace/flake.nix ] && [ -z "$JAIL_AI_NIX_LOADED" ]; then
  export JAIL_AI_NIX_LOADED=1
  echo "🔵 Nix flake detected in /workspace, loading development environment..."
  cd /workspace
  exec nix develop --command zsh
fi
EOFBASHRC

RUN curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR="/usr/local/bin" sh

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

RUN mkdir -p /workspace && chown agent:agent /workspace \
    && mkdir -p /home/agent/.claude /home/agent/.config/.copilot /home/agent/.cursor /home/agent/.gemini /home/agent/.config/codex /home/agent/.gnupg \
    && chown -R agent:agent /home/agent

USER agent
WORKDIR /workspace

ENV SHELL=/bin/zsh

CMD ["/bin/zsh"]
