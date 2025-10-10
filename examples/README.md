# jail-ai Examples

This directory contains example configurations and Containerfiles for jail-ai.

## Custom Project Layer (jail-ai.Containerfile)

The `jail-ai.Containerfile` is an example of a custom layer that can be added to any project. When this file is present in your project root, jail-ai will automatically detect it and build it as an additional layer in the image stack.

### How It Works

1. **Detection**: jail-ai checks for `jail-ai.Containerfile` in the project root
2. **Build Order**: base → language layers → **custom layer** → agent layer
3. **Layer Caching**: The custom layer is built once and cached (tagged as `localhost/jail-ai-custom:<layer-tag>`)
4. **Auto-rebuild**: Rebuilt automatically when the Containerfile content changes or with `--upgrade`

### Usage

1. Copy `jail-ai.Containerfile` to your project root:
   ```bash
   cp examples/jail-ai.Containerfile /path/to/your/project/
   ```

2. Customize it for your project needs:
   ```dockerfile
   ARG BASE_IMAGE
   FROM ${BASE_IMAGE}
   
   USER root
   
   # Install your project-specific tools
   RUN apt-get update && apt-get install -y --no-install-recommends \
       your-package \
       && rm -rf /var/lib/apt/lists/*
   
   USER agent
   WORKDIR /workspace
   ```

3. Run jail-ai as normal - it will automatically detect and build the custom layer:
   ```bash
   jail-ai claude  # The custom layer will be built automatically
   ```

### Important Notes

- **BASE_IMAGE ARG**: Always use `ARG BASE_IMAGE` and `FROM ${BASE_IMAGE}` - jail-ai passes the appropriate base image
- **User Switching**: Switch back to `USER agent` at the end of your Containerfile
- **Workspace**: Set `WORKDIR /workspace` at the end
- **Clean Up**: Remove package caches to keep image size small (`rm -rf /var/lib/apt/lists/*`)

### Image Tags

The custom layer affects the final image tag:

**Without custom layer:**
- `localhost/jail-ai-agent-claude:base-rust-nodejs`

**With custom layer:**
- `localhost/jail-ai-agent-claude:base-rust-nodejs-custom`

This means projects with the same language stack + custom layer will share the same image.

### Force Rebuild

To force rebuild just the custom layer:

```bash
jail-ai claude --upgrade --force-layers custom
```

To rebuild everything including the custom layer:

```bash
jail-ai claude --upgrade
```

### Examples

#### Example 1: Add Project-Specific Tools

```dockerfile
ARG BASE_IMAGE
FROM ${BASE_IMAGE}

USER root

# Install Docker CLI for this project
RUN apt-get update && apt-get install -y --no-install-recommends \
    docker.io \
    && rm -rf /var/lib/apt/lists/*

USER agent
WORKDIR /workspace
```

#### Example 2: Install Specific Versions

```dockerfile
ARG BASE_IMAGE
FROM ${BASE_IMAGE}

USER root

# Install a specific Node.js version via nvm
RUN curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash \
    && . ~/.nvm/nvm.sh \
    && nvm install 18.17.0 \
    && nvm use 18.17.0

USER agent
WORKDIR /workspace
```

#### Example 3: Add Development Tools

```dockerfile
ARG BASE_IMAGE
FROM ${BASE_IMAGE}

USER root

# Install debugging and profiling tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    gdb \
    valgrind \
    strace \
    && rm -rf /var/lib/apt/lists/*

USER agent
WORKDIR /workspace
```

### Troubleshooting

**Issue**: Custom layer not being detected

**Solution**: Ensure the file is named exactly `jail-ai.Containerfile` (case-sensitive) and is in the project root.

---

**Issue**: Build fails with "BASE_IMAGE not defined"

**Solution**: Make sure you include `ARG BASE_IMAGE` before the `FROM` instruction.

---

**Issue**: Changes to Containerfile not reflected

**Solution**: Use `--upgrade` flag to force rebuild:
```bash
jail-ai claude --upgrade
```
