ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Go development environment"

USER root

# Install Go to /usr/local
ARG GO_VERSION=1.23.4
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://go.dev/dl/go${GO_VERSION}.linux-${ARCH}.tar.gz" | tar -C /usr/local -xz \
    && ln -s /usr/local/go/bin/go /usr/local/bin/go \
    && ln -s /usr/local/go/bin/gofmt /usr/local/bin/gofmt

ENV PATH=/usr/local/go/bin:$PATH

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
