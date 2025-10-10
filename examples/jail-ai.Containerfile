# Example custom jail-ai.Containerfile
# This file will be automatically detected and added as a layer
# when present in your project root

ARG BASE_IMAGE
FROM ${BASE_IMAGE}

# Switch to root for installations
USER root

# Install project-specific tools
# Example: Install specific version of a tool
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Add your custom packages here
    vim-nox \
    tmux \
    && rm -rf /var/lib/apt/lists/*

# Install project-specific development tools
# Example: Install a specific npm package globally
# RUN npm install -g <your-package>

# Example: Install a specific Python package
# RUN pip3 install <your-package>

# Example: Install a specific Rust tool
# RUN cargo install <your-tool>

# Set up custom environment variables
ENV CUSTOM_VAR="custom_value"

# Switch back to agent user
USER agent

# Your custom setup commands here
# Example: Create project-specific directories
RUN mkdir -p /home/agent/custom-dir

WORKDIR /workspace
