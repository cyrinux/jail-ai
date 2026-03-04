# Cloud Provider Layers

jail-ai includes optimized layers for AWS and GCP development with pinned package versions to avoid unnecessary rebuilds.

## Version Pinning

All cloud tools have **pinned versions** in their Containerfiles. This ensures:

- **Reproducible builds**: Same Containerfile = same image
- **Efficient caching**: Layers are only rebuilt when versions change
- **Controlled updates**: You decide when to update tools

## AWS Layer (`containerfiles/aws.Containerfile`)

### Pinned Tools

- **AWS CLI**: v2.22.17
- **eksctl**: v0.197.0
- **SAM CLI**: v1.133.0
- **AWS CDK**: v2.175.2
- **Session Manager Plugin**: v1.2.677.0
- **cfn-lint**: v1.22.3
- **rain**: v1.20.1
- **AWS Copilot**: v1.34.0
- **Steampipe**: v1.0.1

### Updating AWS Tools

Edit `containerfiles/aws.Containerfile` and update the `ARG` versions:

```dockerfile
ARG AWS_CLI_VERSION=2.22.17      # Update this
ARG SAM_CLI_VERSION=1.133.0      # Update this
ARG AWS_CDK_VERSION=2.175.2      # Update this
# ... etc
```

When you update any version, the Containerfile hash changes and the layer will rebuild automatically on next use.

## GCP Layer (`containerfiles/gcp.Containerfile`)

### Pinned Tools

- **gcloud CLI**: v503.0.0 (with emulators and auth plugins)
- **Terraform**: v1.9.8
- **Pulumi**: v3.143.0
- **Cloud SQL Proxy**: v2.14.1
- **Skaffold**: v2.15.0
- **kubectl**: v1.32.0
- **Helm**: v3.16.3
- **kpt**: v1.0.0-beta.58

### Updating GCP Tools

Edit `containerfiles/gcp.Containerfile` and update the `ARG` versions:

```dockerfile
ARG GCLOUD_VERSION=503.0.0       # Update this
ARG TERRAFORM_VERSION=1.9.8      # Update this
ARG KUBECTL_VERSION=1.32.0       # Update this
# ... etc
```

## How Rebuilds Work

jail-ai uses **content-based hashing** to detect changes:

1. Each Containerfile is hashed (SHA256)
2. Hash is stored as a label in the built image
3. On subsequent runs, hash is compared
4. If hash differs → rebuild layer
5. If hash matches → reuse cached layer

### Force Rebuild

To force rebuild of cloud layers:

```bash
# Rebuild all layers
jail-ai claude --cloud --upgrade

# Rebuild only AWS layer
jail-ai claude --cloud --upgrade --force-layers aws

# Rebuild only GCP layer
jail-ai claude --cloud --upgrade --force-layers gcp

# Rebuild both cloud layers
jail-ai claude --cloud --upgrade --force-layers aws,gcp
```

## Checking Installed Versions

Inside a jail with cloud layers:

```bash
# AWS versions
aws --version
eksctl version
sam --version
cdk --version

# GCP versions
gcloud version
terraform version
pulumi version
kubectl version --client
```

## Why Pin Versions?

### Before (unpinned):

```dockerfile
# Downloads latest version every time
RUN curl -sSL "https://awscli.amazonaws.com/awscli-exe-linux-x86_64.zip"
```

**Problem**: Even with same Containerfile, package updates trigger rebuilds, wasting time and bandwidth.

### After (pinned):

```dockerfile
ARG AWS_CLI_VERSION=2.22.17
RUN curl -sSL "https://awscli.amazonaws.com/awscli-exe-linux-x86_64-${AWS_CLI_VERSION}.zip"
```

**Benefit**: Layers rebuild **only** when you update the version in the Containerfile.

## Version Labels

Cloud layers include version labels for tracking:

```bash
# Check AWS layer versions
podman image inspect localhost/jail-ai-aws:latest --format '{{.Labels}}'

# Example output:
# ai.jail.aws.cli.version: 2.22.17
# ai.jail.aws.eksctl.version: 0.197.0
# ai.jail.aws.sam.version: 1.133.0
# ai.jail.aws.cdk.version: 2.175.2
```

## Best Practices

1. **Update periodically**: Check for new tool versions monthly
2. **Test before updating**: Verify compatibility with your projects
3. **Use `--upgrade` after version changes**: Force rebuild to pick up new versions
4. **Document breaking changes**: Note any API changes in commit messages

## Finding Latest Versions

### AWS Tools

- **AWS CLI**: https://github.com/aws/aws-cli/blob/v2/CHANGELOG.rst
- **eksctl**: https://github.com/eksctl-io/eksctl/releases
- **SAM CLI**: https://github.com/aws/aws-sam-cli/releases
- **AWS CDK**: https://github.com/aws/aws-cdk/releases
- **cfn-lint**: https://github.com/aws-cloudformation/cfn-lint/releases

### GCP Tools

- **gcloud**: https://cloud.google.com/sdk/docs/release-notes
- **Terraform**: https://github.com/hashicorp/terraform/releases
- **Pulumi**: https://github.com/pulumi/pulumi/releases
- **kubectl**: https://github.com/kubernetes/kubernetes/releases
- **Helm**: https://github.com/helm/helm/releases

## Troubleshooting

### Layer always rebuilds

Check if the Containerfile hash changed:

```bash
# Get current hash from image
podman image inspect localhost/jail-ai-aws:latest \
  --format '{{index .Labels "ai.jail.containerfile.hash"}}'

# Compare with expected hash
# (jail-ai will show mismatches in debug logs)
```

### Old version persists

Force rebuild the layer:

```bash
jail-ai claude --cloud --upgrade --force-layers aws
```

### Version conflict

If a tool version is incompatible, edit the Containerfile to pin a working version, then rebuild.
