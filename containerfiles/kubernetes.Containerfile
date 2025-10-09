ARG BASE_IMAGE=localhost/jail-ai-base:latest
FROM ${BASE_IMAGE}

LABEL maintainer="jail-ai"
LABEL description="jail-ai Kubernetes development environment"

USER root

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

# Install k9s
ARG K9S_VERSION=0.32.7
RUN ARCH=$(dpkg --print-architecture) && \
    K9S_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/derailed/k9s/releases/download/v${K9S_VERSION}/k9s_Linux_${K9S_ARCH}.tar.gz" | tar xz -C /usr/local/bin k9s && \
    chmod +x /usr/local/bin/k9s

# Install kustomize
ARG KUSTOMIZE_VERSION=5.5.0
RUN ARCH=$(dpkg --print-architecture) && \
    KUSTOMIZE_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/kubernetes-sigs/kustomize/releases/download/kustomize%2Fv${KUSTOMIZE_VERSION}/kustomize_v${KUSTOMIZE_VERSION}_linux_${KUSTOMIZE_ARCH}.tar.gz" | tar xz -C /usr/local/bin && \
    chmod +x /usr/local/bin/kustomize

# Install kubectx and kubens
RUN curl -sSL "https://raw.githubusercontent.com/ahmetb/kubectx/master/kubectx" -o /usr/local/bin/kubectx && \
    curl -sSL "https://raw.githubusercontent.com/ahmetb/kubectx/master/kubens" -o /usr/local/bin/kubens && \
    chmod +x /usr/local/bin/kubectx /usr/local/bin/kubens

# Install stern (multi-pod log tailing)
ARG STERN_VERSION=1.31.0
RUN ARCH=$(dpkg --print-architecture) && \
    STERN_ARCH=$([ "$ARCH" = "amd64" ] && echo "amd64" || echo "arm64") && \
    curl -sSL "https://github.com/stern/stern/releases/download/v${STERN_VERSION}/stern_${STERN_VERSION}_linux_${STERN_ARCH}.tar.gz" | tar xz -C /usr/local/bin stern && \
    chmod +x /usr/local/bin/stern

USER agent
WORKDIR /workspace

CMD ["/bin/zsh"]
