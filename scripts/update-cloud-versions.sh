#!/bin/bash
# Update cloud provider tool versions in Containerfiles
# This script fetches the latest versions and updates the ARG declarations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
AWS_CONTAINERFILE="$REPO_ROOT/containerfiles/aws.Containerfile"
GCP_CONTAINERFILE="$REPO_ROOT/containerfiles/gcp.Containerfile"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to fetch latest GitHub release version
fetch_github_version() {
    local repo=$1
    local version=$(curl -sL "https://api.github.com/repos/$repo/releases/latest" | grep -o '"tag_name": "v[^"]*"' | head -1 | sed 's/"tag_name": "v//;s/"//')
    echo "$version"
}

# Function to fetch latest GitHub tag version
fetch_github_tag() {
    local repo=$1
    local pattern=$2
    local version=$(curl -sL "https://api.github.com/repos/$repo/tags" | grep -o "\"name\": \"$pattern[^\"]*\"" | head -1 | sed 's/"name": "//;s/"//')
    echo "$version"
}

# Function to update ARG in Containerfile
update_arg() {
    local file=$1
    local arg_name=$2
    local new_version=$3
    local current_version=$(grep "^ARG ${arg_name}=" "$file" | sed "s/ARG ${arg_name}=//")
    
    if [ -z "$new_version" ]; then
        echo -e "${RED}✗${NC} $arg_name: Failed to fetch version"
        return 1
    fi
    
    if [ "$current_version" = "$new_version" ]; then
        echo -e "${GREEN}✓${NC} $arg_name: Already up to date ($current_version)"
        return 0
    fi
    
    # Update the version
    sed -i "s/^ARG ${arg_name}=.*/ARG ${arg_name}=${new_version}/" "$file"
    echo -e "${YELLOW}↑${NC} $arg_name: $current_version → ${BLUE}$new_version${NC}"
}

echo "=== Updating Cloud Provider Tool Versions ==="
echo

# Check if Containerfiles exist
if [ ! -f "$AWS_CONTAINERFILE" ]; then
    echo -e "${RED}Error: AWS Containerfile not found at $AWS_CONTAINERFILE${NC}"
    exit 1
fi

if [ ! -f "$GCP_CONTAINERFILE" ]; then
    echo -e "${RED}Error: GCP Containerfile not found at $GCP_CONTAINERFILE${NC}"
    exit 1
fi

echo "📦 Updating AWS tools..."
echo "------------------------"

# AWS CLI
echo "Fetching AWS CLI version..."
AWS_CLI_VERSION=$(fetch_github_tag "aws/aws-cli" "2\.")
update_arg "$AWS_CONTAINERFILE" "AWS_CLI_VERSION" "$AWS_CLI_VERSION"

# eksctl
echo "Fetching eksctl version..."
EKSCTL_VERSION=$(fetch_github_version "eksctl-io/eksctl")
update_arg "$AWS_CONTAINERFILE" "EKSCTL_VERSION" "$EKSCTL_VERSION"

# SAM CLI
echo "Fetching SAM CLI version..."
SAM_CLI_VERSION=$(fetch_github_version "aws/aws-sam-cli")
update_arg "$AWS_CONTAINERFILE" "SAM_CLI_VERSION" "$SAM_CLI_VERSION"

# AWS CDK
echo "Fetching AWS CDK version..."
AWS_CDK_VERSION=$(fetch_github_version "aws/aws-cdk")
update_arg "$AWS_CONTAINERFILE" "AWS_CDK_VERSION" "$AWS_CDK_VERSION"

# Session Manager Plugin - use known latest or check S3
echo "Fetching Session Manager Plugin version..."
# Try to get from S3 listing (may not always work)
SESSION_MANAGER_VERSION="1.2.712.0"  # Update this manually when AWS releases new version
echo -e "${YELLOW}⚠${NC}  SESSION_MANAGER_VERSION: Using known version $SESSION_MANAGER_VERSION (verify at https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html)"

# cfn-lint
echo "Fetching cfn-lint version..."
CFN_LINT_VERSION=$(fetch_github_version "aws-cloudformation/cfn-lint")
update_arg "$AWS_CONTAINERFILE" "CFN_LINT_VERSION" "$CFN_LINT_VERSION"

# rain
echo "Fetching rain version..."
RAIN_VERSION=$(fetch_github_version "aws-cloudformation/rain")
update_arg "$AWS_CONTAINERFILE" "RAIN_VERSION" "$RAIN_VERSION"

# AWS Copilot
echo "Fetching AWS Copilot version..."
AWS_COPILOT_VERSION=$(fetch_github_version "aws/copilot-cli")
update_arg "$AWS_CONTAINERFILE" "AWS_COPILOT_VERSION" "$AWS_COPILOT_VERSION"

# Steampipe
echo "Fetching Steampipe version..."
STEAMPIPE_VERSION=$(fetch_github_version "turbot/steampipe")
update_arg "$AWS_CONTAINERFILE" "STEAMPIPE_VERSION" "$STEAMPIPE_VERSION"

echo
echo "🌐 Updating GCP tools..."
echo "------------------------"

# gcloud - use known latest from release notes
echo "Fetching gcloud version..."
GCLOUD_VERSION="513.0.0"  # Update this manually from https://cloud.google.com/sdk/docs/release-notes
echo -e "${YELLOW}⚠${NC}  GCLOUD_VERSION: Using known version $GCLOUD_VERSION (verify at https://cloud.google.com/sdk/docs/release-notes)"

# Terraform - fetch from HashiCorp releases page
echo "Fetching Terraform version..."
TERRAFORM_VERSION=$(curl -sL https://releases.hashicorp.com/terraform/ 2>/dev/null | grep -oP 'terraform_\K[0-9]+\.[0-9]+\.[0-9]+' | head -1)
update_arg "$GCP_CONTAINERFILE" "TERRAFORM_VERSION" "$TERRAFORM_VERSION"

# Pulumi
echo "Fetching Pulumi version..."
PULUMI_VERSION=$(fetch_github_version "pulumi/pulumi")
update_arg "$GCP_CONTAINERFILE" "PULUMI_VERSION" "$PULUMI_VERSION"

# Cloud SQL Proxy
echo "Fetching Cloud SQL Proxy version..."
CLOUD_SQL_PROXY_VERSION=$(fetch_github_version "GoogleCloudPlatform/cloud-sql-proxy")
update_arg "$GCP_CONTAINERFILE" "CLOUD_SQL_PROXY_VERSION" "$CLOUD_SQL_PROXY_VERSION"

# Skaffold
echo "Fetching Skaffold version..."
SKAFFOLD_VERSION=$(fetch_github_version "GoogleContainerTools/skaffold")
update_arg "$GCP_CONTAINERFILE" "SKAFFOLD_VERSION" "$SKAFFOLD_VERSION"

# kubectl
echo "Fetching kubectl version..."
KUBECTL_VERSION=$(curl -sL https://dl.k8s.io/release/stable.txt | sed 's/v//')
update_arg "$GCP_CONTAINERFILE" "KUBECTL_VERSION" "$KUBECTL_VERSION"

# Helm
echo "Fetching Helm version..."
HELM_VERSION=$(fetch_github_version "helm/helm")
update_arg "$GCP_CONTAINERFILE" "HELM_VERSION" "$HELM_VERSION"

# kpt
echo "Fetching kpt version..."
KPT_VERSION=$(fetch_github_version "kptdev/kpt")
update_arg "$GCP_CONTAINERFILE" "KPT_VERSION" "$KPT_VERSION"

echo
echo "=== Summary ==="
echo -e "${GREEN}✓${NC} Version updates complete!"
echo
echo "Next steps:"
echo "1. Review the changes: git diff containerfiles/"
echo "2. Test the builds: cargo build"
echo "3. Rebuild cloud layers: jail-ai claude --cloud --upgrade --force-layers aws,gcp"
echo "4. Commit changes: git add containerfiles/ && git commit -m '⬆️ Update cloud tool versions'"
echo
echo "Note: Some versions require manual updates (marked with ⚠)."
echo "See the output above for links to check these versions."
