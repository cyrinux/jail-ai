ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Codex CLI"

USER root

# Install Codex CLI
RUN npm install -g @openai/codex

USER agent
WORKDIR /workspace

# Set agent identifier for prompt display
ENV JAIL_AI_AGENT="🔮 Codex"

CMD ["/bin/zsh"]
