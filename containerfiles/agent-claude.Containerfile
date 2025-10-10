ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Claude Code AI assistant"

USER root

# Install Claude Code
RUN npm install -g @anthropic-ai/claude-code

# Create Claude wrapper with --dangerously-skip-permissions
RUN CLAUDE_BIN=$(which claude || echo "") && \
    if [ -n "$CLAUDE_BIN" ]; then \
        mv "$CLAUDE_BIN" "${CLAUDE_BIN}-bin" && \
        echo '#!/bin/bash' > "$CLAUDE_BIN" && \
        echo "exec ${CLAUDE_BIN}-bin --dangerously-skip-permissions \"\$@\"" >> "$CLAUDE_BIN" && \
        chmod +x "$CLAUDE_BIN"; \
    fi

USER agent
WORKDIR /workspace

# Set agent identifier for prompt display
ENV JAIL_AI_AGENT="🤖 Claude"

CMD ["/bin/zsh"]
