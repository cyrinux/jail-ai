# Cloud Layers Quick Reference

## 🚀 Quick Commands

```bash
# Update versions to latest
make update-cloud-versions

# Rebuild cloud layers
make rebuild-cloud-layers

# Or manually
./scripts/update-cloud-versions.sh
jail-ai claude --cloud --upgrade --force-layers aws,gcp
```

## 📊 Current Versions

### AWS Tools
| Tool | Version | Update Source |
|------|---------|---------------|
| AWS CLI | 2.34.2 | GitHub Releases |
| eksctl | 0.224.0 | GitHub Releases |
| SAM CLI | 1.154.0 | GitHub Releases |
| AWS CDK | 2.241.0 | GitHub Releases |
| Session Manager | 1.2.712.0 | AWS Documentation |
| cfn-lint | 1.46.0 | GitHub Releases |
| rain | 1.24.3 | GitHub Releases |
| AWS Copilot | 1.34.1 | GitHub Releases |
| Steampipe | 2.4.0 | GitHub Releases |

### GCP Tools
| Tool | Version | Update Source |
|------|---------|---------------|
| gcloud CLI | 513.0.0 | GCP Release Notes |
| Terraform | 1.15.0 | HashiCorp Releases |
| Pulumi | 3.224.0 | GitHub Releases |
| Cloud SQL Proxy | 2.21.1 | GitHub Releases |
| Skaffold | 2.17.2 | GitHub Releases |
| kubectl | 1.35.2 | Kubernetes Releases |
| Helm | 4.1.1 | GitHub Releases |
| kpt | 1.0.0-beta.61 | GitHub Releases |

## 🔄 Update Workflow

1. **Update**: `make update-cloud-versions`
2. **Review**: `git diff containerfiles/`
3. **Rebuild**: `make rebuild-cloud-layers`
4. **Test**: `jail-ai claude --cloud --shell`
5. **Commit**: `git add containerfiles/ && git commit`

## 🎯 How It Works

- **Pinned versions** in Containerfile ARG declarations
- **Hash-based detection** triggers rebuild when versions change
- **Automatic script** fetches latest versions from official sources
- **Efficient caching** reuses layers when unchanged

## ✅ Benefits

- ✅ No unnecessary rebuilds
- ✅ Reproducible builds
- ✅ Latest tool versions
- ✅ One-command updates
- ✅ Full version control

## 📖 Full Documentation

See [cloud-layers.md](cloud-layers.md) for complete details.
