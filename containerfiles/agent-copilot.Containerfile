ARG BASE_IMAGE=localhost/jail-ai-nodejs:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with GitHub Copilot CLI"

USER root

# Install GitHub Copilot CLI
RUN npm install -g @github/copilot

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
