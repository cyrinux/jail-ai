ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Python development environment"

USER root

# Install Python and development tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    python3 \
    python3-pip \
    python3-venv \
    python3-dev \
    && rm -rf /var/lib/apt/lists/* \
    && ln -sf /usr/bin/python3 /usr/bin/python

# Install common Python development tools
RUN pip3 install --no-cache-dir --break-system-packages \
    black \
    pylint \
    mypy \
    pytest \
    poetry

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
