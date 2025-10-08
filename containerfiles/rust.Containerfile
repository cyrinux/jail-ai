ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Rust development environment"

USER root

# Install Rust dependencies (Debian packages)
RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Install Rust and Cargo binaries to system location
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path \
    && rustup default stable \
    && rustup component add clippy rustfmt \
    && chmod -R a+w $RUSTUP_HOME $CARGO_HOME

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
