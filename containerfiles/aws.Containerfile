ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai AWS development environment"

USER root

# Install AWS CLI v2
RUN ARCH=$(dpkg --print-architecture) && \
    AWS_ARCH=$([ "$ARCH" = "amd64" ] && echo "x86_64" || echo "aarch64") && \
    curl -sSL "https://awscli.amazonaws.com/awscli-exe-linux-${AWS_ARCH}.zip" -o awscliv2.zip && \
    unzip -q awscliv2.zip && \
    ./aws/install && \
    rm -rf awscliv2.zip aws

# Install eksctl (Amazon EKS CLI)
ARG EKSCTL_VERSION=0.197.0
RUN ARCH=$(dpkg --print-architecture) && \
    EKSCTL_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/eksctl-io/eksctl/releases/download/v${EKSCTL_VERSION}/eksctl_Linux_${EKSCTL_ARCH}.tar.gz" | tar xz -C /usr/local/bin && \
    chmod +x /usr/local/bin/eksctl

# Install pipx for Python CLI tools (PEP 668 compliant)
RUN apt-get update && \
    apt-get install -y --no-install-recommends pipx && \
    rm -rf /var/lib/apt/lists/*

# Set pipx to install to system-wide location
ENV PIPX_HOME=/opt/pipx
ENV PIPX_BIN_DIR=/usr/local/bin

# Install AWS SAM CLI via pipx
RUN pipx install aws-sam-cli

# Install AWS CDK
RUN npm install -g aws-cdk

# Install Session Manager plugin
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then \
        curl -sSL "https://s3.amazonaws.com/session-manager-downloads/plugin/latest/ubuntu_64bit/session-manager-plugin.deb" -o session-manager-plugin.deb; \
    else \
        curl -sSL "https://s3.amazonaws.com/session-manager-downloads/plugin/latest/ubuntu_arm64/session-manager-plugin.deb" -o session-manager-plugin.deb; \
    fi && \
    dpkg -i session-manager-plugin.deb && \
    rm session-manager-plugin.deb

# Install cfn-lint (CloudFormation linter) via pipx
RUN pipx install cfn-lint

# Install rain (CloudFormation deployment tool)
ARG RAIN_VERSION=1.20.1
RUN ARCH=$(dpkg --print-architecture) && \
    RAIN_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/aws-cloudformation/rain/releases/download/v${RAIN_VERSION}/rain-v${RAIN_VERSION}_linux-${RAIN_ARCH}.zip" -o rain.zip && \
    unzip -q rain.zip && \
    mv rain-v${RAIN_VERSION}_linux-${RAIN_ARCH}/rain /usr/local/bin/rain && \
    rm -rf rain.zip rain-v${RAIN_VERSION}_linux-${RAIN_ARCH} && \
    chmod +x /usr/local/bin/rain

# Install AWS Copilot CLI (for ECS/App Runner deployments)
ARG AWS_COPILOT_VERSION=1.34.0
RUN ARCH=$(dpkg --print-architecture) && \
    COPILOT_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL --http1.1 "https://github.com/aws/copilot-cli/releases/download/v${AWS_COPILOT_VERSION}/copilot-linux-${COPILOT_ARCH}" -o /usr/local/bin/copilot && \
    chmod +x /usr/local/bin/copilot

# Install Steampipe (for AWS resource querying)
ARG STEAMPIPE_VERSION=1.0.1
RUN ARCH=$(dpkg --print-architecture) && \
    STEAMPIPE_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/turbot/steampipe/releases/download/v${STEAMPIPE_VERSION}/steampipe_linux_${STEAMPIPE_ARCH}.tar.gz" -o steampipe.tar.gz && \
    tar xzf steampipe.tar.gz -C /usr/local/bin steampipe && \
    rm steampipe.tar.gz && \
    chmod +x /usr/local/bin/steampipe

USER agent
WORKDIR /workspace

# Install AWS Steampipe plugin for agent user
RUN steampipe plugin install aws || true

CMD ["/bin/zsh"]
