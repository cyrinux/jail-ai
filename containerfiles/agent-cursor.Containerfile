ARG BASE_IMAGE=localhost/jail-ai-nodejs:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Cursor Agent CLI"

USER root

# Install Cursor Agent CLI with default installation (works natively with Debian/glibc)
RUN curl -fsSL https://cursor.com/install | bash

# Add cursor to PATH (default install location is ~/.local/bin)
ENV PATH="/root/.local/bin:${PATH}"

USER agent

# Ensure agent user also has cursor in PATH
ENV PATH="/home/agent/.local/bin:${PATH}"

WORKDIR /workspace

CMD ["/bin/zsh"]
