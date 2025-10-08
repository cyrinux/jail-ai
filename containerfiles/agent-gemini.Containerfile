ARG BASE_IMAGE=localhost/jail-ai-nodejs:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Gemini CLI"

USER root

# Install Gemini CLI
RUN npm install -g @google/gemini-cli

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
