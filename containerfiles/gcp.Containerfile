ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Google Cloud Platform development environment"

# Version pins for reproducible builds
# Update these versions to trigger layer rebuilds
ARG GCLOUD_VERSION=513.0.0
ARG TERRAFORM_VERSION=1.15.0
ARG PULUMI_VERSION=3.224.0
ARG CLOUD_SQL_PROXY_VERSION=2.21.1
ARG SKAFFOLD_VERSION=2.17.2
ARG KUBECTL_VERSION=1.35.2
ARG HELM_VERSION=4.1.1
ARG KPT_VERSION=1.0.0-beta.61

LABEL ai.jail.gcp.gcloud.version="${GCLOUD_VERSION}"
LABEL ai.jail.gcp.terraform.version="${TERRAFORM_VERSION}"
LABEL ai.jail.gcp.pulumi.version="${PULUMI_VERSION}"
LABEL ai.jail.gcp.kubectl.version="${KUBECTL_VERSION}"

USER root

# Install Google Cloud CLI and components (pinned version)
RUN curl -fsSL https://packages.cloud.google.com/apt/doc/apt-key.gpg | gpg --dearmor -o /usr/share/keyrings/cloud.google.gpg \
    && echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | tee /etc/apt/sources.list.d/google-cloud-sdk.list > /dev/null \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        google-cloud-cli=${GCLOUD_VERSION}-0 \
        google-cloud-cli-gke-gcloud-auth-plugin=${GCLOUD_VERSION}-0 \
        google-cloud-cli-cloud-run-proxy=${GCLOUD_VERSION}-0 \
        google-cloud-cli-firestore-emulator=${GCLOUD_VERSION}-0 \
        google-cloud-cli-pubsub-emulator=${GCLOUD_VERSION}-0 \
        google-cloud-cli-bigtable-emulator=${GCLOUD_VERSION}-0 \
        google-cloud-cli-datastore-emulator=${GCLOUD_VERSION}-0 \
    && rm -rf /var/lib/apt/lists/*

# Install Terraform (for GCP infrastructure)
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://releases.hashicorp.com/terraform/${TERRAFORM_VERSION}/terraform_${TERRAFORM_VERSION}_linux_${ARCH}.zip" -o terraform.zip && \
    unzip -q terraform.zip -d /usr/local/bin && \
    rm terraform.zip && \
    chmod +x /usr/local/bin/terraform

# Install Pulumi (alternative IaC for GCP) - pinned version
RUN curl -fsSL https://get.pulumi.com | sh -s -- --version ${PULUMI_VERSION} --install-root /usr/local

# Install Cloud SQL Proxy (pinned version)
RUN ARCH=$(dpkg --print-architecture) && \
    PROXY_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://storage.googleapis.com/cloud-sql-connectors/cloud-sql-proxy/v${CLOUD_SQL_PROXY_VERSION}/cloud-sql-proxy.linux.${PROXY_ARCH}" -o /usr/local/bin/cloud-sql-proxy && \
    chmod +x /usr/local/bin/cloud-sql-proxy

# Install Skaffold (for GKE development) - pinned version
RUN ARCH=$(dpkg --print-architecture) && \
    SKAFFOLD_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://storage.googleapis.com/skaffold/releases/v${SKAFFOLD_VERSION}/skaffold-linux-${SKAFFOLD_ARCH}" -o /usr/local/bin/skaffold && \
    chmod +x /usr/local/bin/skaffold

# Install kubectl (pinned version)
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://dl.k8s.io/release/v${KUBECTL_VERSION}/bin/linux/${ARCH}/kubectl" -o /usr/local/bin/kubectl && \
    chmod +x /usr/local/bin/kubectl

# Install Helm
RUN ARCH=$(dpkg --print-architecture) && \
    curl -sSL "https://get.helm.sh/helm-v${HELM_VERSION}-linux-${ARCH}.tar.gz" | tar xz && \
    mv linux-${ARCH}/helm /usr/local/bin/helm && \
    rm -rf linux-${ARCH} && \
    chmod +x /usr/local/bin/helm

# Install kpt (Kubernetes Resource Model tool) - pinned version
RUN ARCH=$(dpkg --print-architecture) && \
    KPT_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/kptdev/kpt/releases/download/v${KPT_VERSION}/kpt_linux_${KPT_ARCH}" -o /usr/local/bin/kpt && \
    chmod +x /usr/local/bin/kpt

# Install Config Connector (KCC) CLI - using latest (no stable versioning)
RUN ARCH=$(dpkg --print-architecture) && \
    KCC_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/GoogleCloudPlatform/k8s-config-connector/releases/latest/download/cli_linux_${KCC_ARCH}" -o /usr/local/bin/config-connector && \
    chmod +x /usr/local/bin/config-connector || true

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
