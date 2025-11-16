ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Claude Code Router"

USER root

# Install Claude Code (required by Claude Code Router) and Claude Code Router
RUN npm install -g @anthropic-ai/claude-code @musistudio/claude-code-router

USER agent
WORKDIR /workspace

# Set agent identifier for prompt display
ENV JAIL_AI_AGENT="🔀 ClaudeCodeRouter"

CMD ["/bin/zsh"]
