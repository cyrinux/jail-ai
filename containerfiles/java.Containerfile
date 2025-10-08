ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Java development environment"

USER root

# Install OpenJDK and build tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    default-jdk \
    maven \
    gradle \
    && rm -rf /var/lib/apt/lists/*

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
