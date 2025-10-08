ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Python development environment"

USER root

# Install Python and related tools
RUN apk add --no-cache \
    python3 \
    python3-dev \
    py3-pip \
    && ln -sf /usr/bin/python3 /usr/bin/python

# Install common Python development tools
RUN pip3 install --no-cache-dir --break-system-packages \
    black \
    pylint \
    mypy \
    pytest \
    poetry

# Add Poetry to PATH
ENV PATH="/root/.local/bin:${PATH}"

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
