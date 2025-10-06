# jail-ai Configuration

This directory contains the default Containerfile used to build the `localhost/jail-ai-env:latest` image.

## Customization

The `Containerfile` in this directory will be automatically copied to `~/.config/jail-ai/Containerfile` on first use.

To customize the image:

1. Edit `~/.config/jail-ai/Containerfile` to add or modify tools, languages, or configurations
2. The next time you create a jail using the default image, jail-ai will automatically detect the changes and rebuild the image

## Automatic Rebuilding

jail-ai tracks changes to the Containerfile using SHA256 hashing:
- When you run `jail-ai create` or `jail-ai claude`, the tool checks if:
  - The default image exists locally
  - The Containerfile has changed since the last build
- If either condition is true, the image is automatically rebuilt

## Manual Building

You can also manually build the image:

```bash
# Build with podman
podman build -t localhost/jail-ai-env:latest -f ~/.config/jail-ai/Containerfile ~/.config/jail-ai/

# Or use the Makefile from the repository root
make build-image
```

## Location

- **Default Containerfile**: Embedded in the jail-ai binary
- **User Containerfile**: `~/.config/jail-ai/Containerfile` (or `$XDG_CONFIG_HOME/jail-ai/Containerfile`)
- **Hash Cache**: `~/.config/jail-ai/.containerfile.sha256`

## Custom Images

If you want to use a completely different image instead of the default jail-ai image:

```bash
# Use any OCI image
jail-ai create my-jail --image alpine:latest
jail-ai create my-jail --image docker.io/ubuntu:22.04

# For custom images, jail-ai will pull them if not available locally
# but won't attempt to build them
```
