ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Jules CLI"

USER root

# Install Jules CLI
RUN npm install -g @google/jules


USER agent
WORKDIR /workspace

# Set agent identifier for prompt display
ENV JAIL_AI_AGENT="🤖 Jules"

CMD ["/bin/zsh"]
