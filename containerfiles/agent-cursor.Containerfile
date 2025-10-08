ARG BASE_IMAGE=localhost/jail-ai-nodejs:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Cursor Agent CLI"

USER root

# Install Cursor Agent CLI to system location
RUN curl -fsSL https://cursor.com/install | bash \
    && if [ -d /root/.local/share/cursor-agent ]; then \
        mkdir -p /usr/local/share && \
        mv /root/.local/share/cursor-agent /usr/local/share/cursor-agent && \
        CURSOR_BIN=$(find /usr/local/share/cursor-agent/versions -name cursor-agent -type f | head -n1) && \
        if [ -n "$CURSOR_BIN" ]; then \
            ln -s "$CURSOR_BIN" /usr/local/bin/cursor-agent; \
        fi; \
    fi

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
