ARG BASE_IMAGE=localhost/jail-ai-nodejs:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Codex CLI"

USER root

# Install Codex CLI
RUN npm install -g @openai/codex

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
