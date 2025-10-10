ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Gemini CLI"

USER root

# Install Gemini CLI
RUN npm install -g @google/gemini-cli

USER agent
WORKDIR /workspace

# Set agent identifier for prompt display
ENV JAIL_AI_AGENT="💎 Gemini"

CMD ["/bin/zsh"]
