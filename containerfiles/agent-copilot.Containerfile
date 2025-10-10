ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with GitHub Copilot CLI"

USER root

# Install GitHub Copilot CLI
RUN npm install -g @github/copilot

USER agent
WORKDIR /workspace

# Set agent identifier for prompt display
ENV JAIL_AI_AGENT="🐙 Copilot"

CMD ["/bin/zsh"]
