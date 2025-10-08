ARG BASE_IMAGE=localhost/jail-ai-nodejs:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Cursor Agent CLI"

USER root

# Install Cursor Agent CLI to system location
# Note: Cursor bundles its own node binary (glibc), but we're on Alpine (musl)
# Solution: Replace bundled node with wrapper that filters incompatible flags
RUN curl -fsSL https://cursor.com/install | bash \
    && if [ -d /root/.local/share/cursor-agent ]; then \
        mkdir -p /usr/local/share && \
        mv /root/.local/share/cursor-agent /usr/local/share/cursor-agent && \
        CURSOR_BIN=$(find /usr/local/share/cursor-agent/versions -name cursor-agent -type f | head -n1) && \
        if [ -n "$CURSOR_BIN" ]; then \
            ln -s "$CURSOR_BIN" /usr/local/bin/cursor-agent && \
            CURSOR_VERSION_DIR=$(dirname "$CURSOR_BIN") && \
            if [ -f "$CURSOR_VERSION_DIR/node" ]; then \
                mv "$CURSOR_VERSION_DIR/node" "$CURSOR_VERSION_DIR/node-bundled" && \
                echo '#!/bin/sh' > "$CURSOR_VERSION_DIR/node" && \
                echo '# Wrapper to filter out Alpine-incompatible node flags' >> "$CURSOR_VERSION_DIR/node" && \
                echo 'FILTERED_ARGS=""' >> "$CURSOR_VERSION_DIR/node" && \
                echo 'for arg in "$@"; do' >> "$CURSOR_VERSION_DIR/node" && \
                echo '  case "$arg" in' >> "$CURSOR_VERSION_DIR/node" && \
                echo '    --use-system-ca) ;;' >> "$CURSOR_VERSION_DIR/node" && \
                echo '    *) FILTERED_ARGS="$FILTERED_ARGS $arg" ;;' >> "$CURSOR_VERSION_DIR/node" && \
                echo '  esac' >> "$CURSOR_VERSION_DIR/node" && \
                echo 'done' >> "$CURSOR_VERSION_DIR/node" && \
                echo 'exec /usr/bin/node $FILTERED_ARGS' >> "$CURSOR_VERSION_DIR/node" && \
                chmod +x "$CURSOR_VERSION_DIR/node"; \
            fi; \
        fi; \
    fi

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
