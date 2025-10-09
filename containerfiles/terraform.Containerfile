ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Terraform/OpenTofu development environment"

USER root

# Install Terraform
ARG TERRAFORM_VERSION=1.9.8
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_${ARCH}.zip" -o terraform.zip && \
    unzip terraform.zip -d /usr/local/bin && \
    rm terraform.zip && \
    chmod +x /usr/local/bin/terraform

# Install OpenTofu (open-source Terraform fork)
ARG TOFU_VERSION=1.8.5
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://github.com/opentofu/opentofu/releases/download/v${TOFU_VERSION}/tofu_${TOFU_VERSION}_linux_${ARCH}.zip" -o tofu.zip && \
    unzip tofu.zip -d /usr/local/bin && \
    rm tofu.zip && \
    chmod +x /usr/local/bin/tofu

# Install Terragrunt
ARG TERRAGRUNT_VERSION=0.68.16
RUN ARCH=$(dpkg --print-architecture) && \
    TERRAGRUNT_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/gruntwork-io/terragrunt/releases/download/v${TERRAGRUNT_VERSION}/terragrunt_linux_${TERRAGRUNT_ARCH}" -o /usr/local/bin/terragrunt && \
    chmod +x /usr/local/bin/terragrunt

# Install tflint
ARG TFLINT_VERSION=0.54.0
RUN curl -sSL "https://github.com/terraform-linters/tflint/releases/download/v${TFLINT_VERSION}/tflint_linux_$(dpkg --print-architecture).zip" -o tflint.zip && \
    unzip tflint.zip -d /usr/local/bin && \
    rm tflint.zip && \
    chmod +x /usr/local/bin/tflint

# Install terraform-docs
ARG TFDOCS_VERSION=0.19.0
RUN ARCH=$(dpkg --print-architecture) && \
    TFDOCS_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/terraform-docs/terraform-docs/releases/download/v${TFDOCS_VERSION}/terraform-docs-v${TFDOCS_VERSION}-linux-${TFDOCS_ARCH}.tar.gz" | tar xz -C /usr/local/bin terraform-docs && \
    chmod +x /usr/local/bin/terraform-docs

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
