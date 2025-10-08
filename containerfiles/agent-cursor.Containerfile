ARG BASE_IMAGE=localhost/jail-ai-nodejs:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai with Cursor Agent CLI"

USER root

# Install Cursor Agent CLI with default installation (works natively with Debian/glibc)
# Install to /root, then symlink to system location for all users
RUN curl -fsSL https://cursor.com/install | bash \
    && ln -sf /root/.local/bin/cursor-agent /usr/local/bin/cursor-agent

USER agent

WORKDIR /workspace

CMD ["/bin/zsh"]
