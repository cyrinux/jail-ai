ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Google Cloud Platform development environment"

USER root

# Install Google Cloud CLI and components
RUN curl -fsSL https://packages.cloud.google.com/apt/doc/apt-key.gpg | gpg --dearmor -o /usr/share/keyrings/cloud.google.gpg \
    && echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | tee /etc/apt/sources.list.d/google-cloud-sdk.list > /dev/null \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        google-cloud-cli \
        google-cloud-cli-gke-gcloud-auth-plugin \
        google-cloud-cli-cloud-run-proxy \
        google-cloud-cli-firestore-emulator \
        google-cloud-cli-pubsub-emulator \
        google-cloud-cli-bigtable-emulator \
        google-cloud-cli-datastore-emulator \
    && rm -rf /var/lib/apt/lists/*

# Install Terraform (for GCP infrastructure)
ARG TERRAFORM_VERSION=1.9.8
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_${ARCH}.zip" -o terraform.zip && \
    unzip -q terraform.zip -d /usr/local/bin && \
    rm terraform.zip && \
    chmod +x /usr/local/bin/terraform

# Install Pulumi (alternative IaC for GCP)
RUN curl -fsSL https://get.pulumi.com | sh -s -- --install-root /usr/local

# Install Cloud SQL Proxy
RUN ARCH=$(dpkg --print-architecture) && \
    PROXY_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://storage.googleapis.com/cloud-sql-connectors/cloud-sql-proxy/v2.14.1/cloud-sql-proxy.linux.${PROXY_ARCH}" -o /usr/local/bin/cloud-sql-proxy && \
    chmod +x /usr/local/bin/cloud-sql-proxy

# Install Skaffold (for GKE development)
RUN ARCH=$(dpkg --print-architecture) && \
    SKAFFOLD_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://storage.googleapis.com/skaffold/releases/latest/skaffold-linux-${SKAFFOLD_ARCH}" -o /usr/local/bin/skaffold && \
    chmod +x /usr/local/bin/skaffold

# Install kubectl
RUN curl -sSL "https://dl.k8s.io/release/$(curl -sSL https://dl.k8s.io/release/stable.txt)/bin/linux/$(dpkg --print-architecture)/kubectl" -o /usr/local/bin/kubectl && \
    chmod +x /usr/local/bin/kubectl

# Install Helm
ARG HELM_VERSION=3.16.3
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://get.helm.sh/helm-v${HELM_VERSION}-linux-${ARCH}.tar.gz" | tar xz && \
    mv linux-${ARCH}/helm /usr/local/bin/helm && \
    rm -rf linux-${ARCH} && \
    chmod +x /usr/local/bin/helm

# Install kpt (Kubernetes Resource Model tool)
RUN ARCH=$(dpkg --print-architecture) && \
    KPT_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/kptdev/kpt/releases/latest/download/kpt_linux_${KPT_ARCH}" -o /usr/local/bin/kpt && \
    chmod +x /usr/local/bin/kpt

# Install Config Connector (KCC) CLI
RUN ARCH=$(dpkg --print-architecture) && \
    KCC_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/GoogleCloudPlatform/k8s-config-connector/releases/latest/download/cli_linux_${KCC_ARCH}" -o /usr/local/bin/config-connector && \
    chmod +x /usr/local/bin/config-connector || true

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
