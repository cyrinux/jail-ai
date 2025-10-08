ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Cursor Agent CLI"

USER root

# Install Cursor Agent CLI to system location
# Cursor installer creates versioned dir in ~/.local/share/cursor-agent and symlink in ~/.local/bin
# Move entire installation to /usr/local, replace bundled node with system node
RUN curl -fsSL https://cursor.com/install | bash \
    && if [ -d /root/.local/share/cursor-agent ]; then \
        mkdir -p /usr/local/share && \
        mv /root/.local/share/cursor-agent /usr/local/share/cursor-agent && \
        CURSOR_BIN=$(find /usr/local/share/cursor-agent/versions -name cursor-agent -type f | head -n1) && \
        if [ -n "$CURSOR_BIN" ]; then \
            ln -s "$CURSOR_BIN" /usr/local/bin/cursor-agent && \
            CURSOR_VERSION_DIR=$(dirname "$CURSOR_BIN") && \
            if [ -f "$CURSOR_VERSION_DIR/node" ]; then \
                rm "$CURSOR_VERSION_DIR/node" && \
                ln -s "$(which node)" "$CURSOR_VERSION_DIR/node"; \
            fi; \
        fi; \
    fi

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
