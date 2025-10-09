ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai C/C++ development environment"

USER root

# Install C/C++ compilers and development tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    # Compilers
    gcc \
    g++ \
    clang \
    # Build tools
    cmake \
    make \
    ninja-build \
    autoconf \
    automake \
    libtool \
    # Development libraries
    libssl-dev \
    zlib1g-dev \
    libreadline-dev \
    libsqlite3-dev \
    # Debugging tools
    gdb \
    valgrind \
    # Code analysis
    cppcheck \
    clang-format \
    clang-tidy \
    # Documentation
    doxygen \
    && rm -rf /var/lib/apt/lists/*

# Install vcpkg (C/C++ package manager)
USER agent
# RUN git clone --depth=1 https://github.com/microsoft/vcpkg.git /home/agent/.vcpkg \
#   && /home/agent/.vcpkg/bootstrap-vcpkg.sh -disableMetrics

ENV PATH="/home/agent/.vcpkg:$PATH"

WORKDIR /workspace

CMD ["/bin/zsh"]
