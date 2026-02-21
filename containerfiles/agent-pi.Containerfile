ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Pi"

USER root

# Install Pi
RUN npm install -g @mariozechner/pi-coding-agent

USER agent
WORKDIR /workspace

# Set agent identifier for prompt display
ENV JAIL_AI_AGENT="🤖 pi"

CMD ["/bin/zsh"]
