ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Node.js development environment"

USER root

# Install Node.js and npm (LTS version from Alpine packages)
RUN apk add --no-cache \
    nodejs \
    npm

# Install yarn and pnpm globally
RUN npm install -g yarn pnpm

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
