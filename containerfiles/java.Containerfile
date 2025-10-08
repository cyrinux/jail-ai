ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Java development environment"

USER root

# Install Java (OpenJDK)
RUN apk add --no-cache \
    openjdk21 \
    maven \
    gradle

ENV JAVA_HOME=/usr/lib/jvm/java-21-openjdk

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
