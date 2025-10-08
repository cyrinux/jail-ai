ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Go development environment"

USER root

# Install Go
ARG GO_VERSION=1.23.4
ENV PATH="/usr/local/go/bin:${PATH}"

RUN ARCH=$(uname -m) && \
    case "$ARCH" in \
        x86_64) GOARCH=amd64 ;; \
        aarch64) GOARCH=arm64 ;; \
        armv7l) GOARCH=armv6l ;; \
        *) echo "Unsupported architecture: $ARCH" && exit 1 ;; \
    esac && \
    wget -q "https://go.dev/dl/go${GO_VERSION}.linux-${GOARCH}.tar.gz" && \
    tar -C /usr/local -xzf "go${GO_VERSION}.linux-${GOARCH}.tar.gz" && \
    rm "go${GO_VERSION}.linux-${GOARCH}.tar.gz" && \
    ln -s /usr/local/go/bin/go /usr/local/bin/go && \
    ln -s /usr/local/go/bin/gofmt /usr/local/bin/gofmt

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
