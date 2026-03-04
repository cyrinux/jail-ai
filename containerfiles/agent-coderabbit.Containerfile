ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with CodeRabbit AI code review assistant"

USER root

# Install CodeRabbit CLI
RUN curl -fsSL https://cli.coderabbit.ai/install.sh | sh

USER agent
WORKDIR /workspace

# Set agent identifier for prompt display
ENV JAIL_AI_AGENT="🐰 CodeRabbit"

CMD ["/bin/zsh"]
