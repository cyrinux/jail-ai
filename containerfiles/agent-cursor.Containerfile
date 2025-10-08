# cursor-agent requires glibc (has native .node addons), so we use Debian instead of Alpine
FROM docker.io/library/node:lts-slim

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Cursor Agent CLI (Debian-based for glibc compatibility)"

# Build arguments for user/group ID mapping
ARG PUID=1000
ARG PGID=1000

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

# Install minimal required tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    bash \
    zsh \
    curl \
    ca-certificates \
    git \
    sudo \
    && rm -rf /var/lib/apt/lists/*

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

# Install Cursor Agent CLI (native addons work with glibc)
RUN curl -fsSL https://cursor.com/install | bash \
    && if [ -d /root/.local/share/cursor-agent ]; then \
        mkdir -p /usr/local/share && \
        mv /root/.local/share/cursor-agent /usr/local/share/cursor-agent && \
        CURSOR_BIN=$(find /usr/local/share/cursor-agent/versions -name cursor-agent -type f | head -n1) && \
        if [ -n "$CURSOR_BIN" ]; then \
            ln -s "$CURSOR_BIN" /usr/local/bin/cursor-agent; \
        fi; \
    fi

# Create workspace directory
RUN mkdir -p /workspace && chown agent:agent /workspace \
    && mkdir -p /home/agent/.cursor /home/agent/.config/cursor \
    && chown -R agent:agent /home/agent

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
