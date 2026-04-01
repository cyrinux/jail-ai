ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with OpenCode CLI"

USER root

RUN npm install -g opencode-ai

USER agent
WORKDIR /workspace

ENV JAIL_AI_AGENT="🤖 OpenCode"

CMD ["/bin/zsh"]
