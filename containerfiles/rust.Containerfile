ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Rust development environment"

USER root

# Install Rust system dependencies (for building native extensions)
RUN apk add --no-cache \
    linux-headers \
    musl-dev

# Install Rust and Cargo binaries to system location
ENV RUSTUP_HOME="/usr/local/rustup" \
    PATH="/usr/local/cargo/bin:${PATH}"

RUN export CARGO_HOME=/usr/local/cargo \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
    && $CARGO_HOME/bin/rustup default stable \
    && $CARGO_HOME/bin/rustup component add clippy rustfmt

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
