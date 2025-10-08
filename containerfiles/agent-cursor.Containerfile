ARG BASE_IMAGE=localhost/jail-ai-nodejs:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Cursor Agent CLI"

USER agent

# Install Cursor Agent CLI with default installation (works natively with Debian/glibc)
# Install as agent user so it's in the agent's home directory
RUN curl -fsSL https://cursor.com/install | bash

# Add cursor to PATH
ENV PATH="/home/agent/.local/bin:${PATH}"

WORKDIR /workspace

CMD ["/bin/zsh"]
