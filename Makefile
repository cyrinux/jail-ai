.PHONY: help build-image push-image test-image clean build build-ebpf build-loader install-loader build-all clean-ebpf run test clippy fmt install-man uninstall-man view-man

IMAGE_NAME ?= localhost/jail-ai-env
IMAGE_TAG ?= latest
IMAGE_FULL = $(IMAGE_NAME):$(IMAGE_TAG)
PREFIX ?= /usr/local
DESTDIR ?=

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build-image: ## Build the AI agent environment container image (optional - jail-ai auto-builds)
	@echo "Building container image: $(IMAGE_FULL)"
	@echo "Note: jail-ai now uses a layered image system and auto-builds images when needed."
	@echo "This target is kept for backward compatibility but may not reflect the layered architecture."
	@echo "The layered system automatically detects your project type and builds appropriate images."
	@echo "To force rebuild layers, use: cargo run -- <agent> --upgrade"
	podman build -t $(IMAGE_FULL) -f containerfiles/base.Containerfile \
		--build-arg PUID=$$(id -u) \
		--build-arg PGID=$$(id -g) \
		containerfiles/
	@echo "Image built successfully: $(IMAGE_FULL)"

push-image: ## Push the container image to a registry
	@echo "Pushing image: $(IMAGE_FULL)"
	podman push $(IMAGE_FULL)

test-image: ## Test the container image
	@echo "Testing image: $(IMAGE_FULL)"
	@podman run --rm --user agent $(IMAGE_FULL) bash --version
	@podman run --rm --user agent $(IMAGE_FULL) zsh --version
	@podman run --rm --user agent $(IMAGE_FULL) fzf --version
	@podman run --rm --user agent $(IMAGE_FULL) rg --version
	@podman run --rm --user agent $(IMAGE_FULL) cargo --version
	@podman run --rm --user agent $(IMAGE_FULL) go version
	@podman run --rm --user agent $(IMAGE_FULL) node --version
	@podman run --rm --user agent $(IMAGE_FULL) python --version
	@podman run --rm --user agent $(IMAGE_FULL) claude --version
	@podman run --rm --user agent $(IMAGE_FULL) copilot --version || echo "Copilot CLI installed (requires auth)"
	@podman run --rm --user agent $(IMAGE_FULL) cursor-agent --version || echo "Cursor Agent installed (requires auth)"
	@podman run --rm --user agent $(IMAGE_FULL) gemini --version || echo "Gemini CLI installed (requires auth)"
	@podman run --rm --user agent $(IMAGE_FULL) codex --version || echo "Codex CLI installed (requires auth)"
	@podman run --rm --user agent $(IMAGE_FULL) jules --version || echo "Jules CLI installed (requires auth)"
	@echo "All tools verified successfully!"

clean: ## Remove built container images and eBPF build container
	@echo "Removing image: $(IMAGE_FULL)"
	-podman rmi $(IMAGE_FULL)
	@$(MAKE) clean-ebpf

build: ## Build the jail-ai binary
	cargo build --release

install: build ## Install the jail-ai binary
	cargo install --path .

build-loader: ## Build the eBPF loader helper binary
	@echo "Building jail-ai-ebpf-loader..."
	cargo build --release -p jail-ai-ebpf-loader
	@echo "✓ Helper binary built at: target/release/jail-ai-ebpf-loader"

install-loader: build-loader ## Install the eBPF loader helper binary with capabilities
	@echo "Installing jail-ai-ebpf-loader..."
	cargo install --path jail-ai-ebpf-loader --force
	@echo "✓ Helper binary installed"
	@echo ""
	@echo "⚠️  SECURITY NOTICE: Granting capabilities to helper binary"
	@echo "This gives CAP_BPF and CAP_NET_ADMIN to the small (~400 LOC) helper binary."
	@echo "The main jail-ai binary remains unprivileged."
	@echo ""
	sudo setcap cap_bpf,cap_net_admin+ep $$(which jail-ai-ebpf-loader)
	@echo "✓ Capabilities granted to helper binary"
	@echo ""
	@echo "Verification:"
	@echo "  Helper capabilities: $$(getcap $$(which jail-ai-ebpf-loader))"

build-ebpf: ## Build eBPF programs in a container (reuses container if exists)
	@echo "Building eBPF programs..."
	./build-ebpf.sh

build-all: build-ebpf build build-loader ## Build everything (eBPF programs, main binary, and loader)

clean-ebpf: ## Remove the eBPF build container
	@echo "Removing eBPF build container..."
	@if command -v podman &> /dev/null; then \
		podman rm -f build-ebpf 2>/dev/null || true; \
	elif command -v docker &> /dev/null; then \
		docker rm -f build-ebpf 2>/dev/null || true; \
	fi
	@echo "eBPF build container removed."

run: ## Run jail-ai (use ARGS="..." to pass arguments)
	cargo run -- $(ARGS)

test: ## Run tests
	cargo test

clippy: ## Run clippy lints
	cargo clippy -- -D warnings

fmt: ## Format code
	cargo fmt

dev-jail: ## Create a development jail (image auto-built if needed)
	cargo run -- create dev-agent --no-network

# Example usage targets
.PHONY: example-create example-exec example-remove example-claude example-copilot example-cursor example-gemini example-codex example-jules

example-create: ## Example: Create a jail (image auto-built if needed)
	cargo run -- create my-agent

example-exec: ## Example: Execute a command in the jail
	cargo run -- exec my-agent -- ls -la /workspace

example-remove: ## Example: Remove the jail
	cargo run -- remove my-agent --force

example-claude: ## Example: Run Claude agent
	cargo run -- claude -- chat "help me debug this code"

example-copilot: ## Example: Run Copilot agent
	cargo run -- copilot --copilot-dir -- suggest "write tests"

example-cursor: ## Example: Run Cursor agent
	cargo run -- cursor --cursor-dir -- --help

example-gemini: ## Example: Run Gemini agent
	cargo run -- gemini --gemini-dir -- --model gemini-pro "explain this"

example-codex: ## Example: Run Codex agent (use --auth for first-time setup)
	cargo run -- codex --codex-dir -- generate "create a REST API"

example-jules: ## Example: Run Jules agent (use --auth for first-time setup)
	cargo run -- jules --jules-dir -- chat "help with this code"

# Advanced feature examples
.PHONY: example-upgrade example-isolated example-shell example-auth example-git-gpg example-agent-configs example-no-nix

example-upgrade: ## Example: Upgrade jail with latest image layers
	cargo run -- claude --upgrade

example-isolated: ## Example: Create isolated project-specific jail
	cargo run -- claude --isolated

example-shell: ## Example: Open interactive shell in jail
	cargo run -- claude --shell

example-auth: ## Example: Authenticate Codex (OAuth workflow)
	cargo run -- codex --codex-dir --auth

example-git-gpg: ## Example: Create jail with git and GPG config
	cargo run -- claude --git-gpg

example-agent-configs: ## Example: Mount all agent config directories
	cargo run -- claude --agent-configs

example-no-nix: ## Example: Skip nix layer and use other language layers
	cargo run -- claude --no-nix

# Quick test targets
.PHONY: test-claude test-copilot test-cursor test-gemini test-codex test-jules test-all-agents

test-claude: ## Quick test: Claude agent
	cargo run -- claude -- --version

test-copilot: ## Quick test: Copilot agent
	cargo run -- copilot --copilot-dir -- --version

test-cursor: ## Quick test: Cursor agent
	cargo run -- cursor --cursor-dir -- --version

test-gemini: ## Quick test: Gemini agent
	cargo run -- gemini --gemini-dir -- --version

test-codex: ## Quick test: Codex agent
	cargo run -- codex --codex-dir -- --version

test-jules: ## Quick test: Jules agent
	cargo run -- jules --jules-dir -- --version

test-all-agents: test-claude test-copilot test-cursor test-gemini test-codex test-jules ## Test all agents

# Man page targets
.PHONY: install-man uninstall-man view-man

install-man: docs/jail-ai.1 ## Install man page system-wide
	@echo "Installing man page to $(DESTDIR)$(PREFIX)/share/man/man1/"
	mkdir -p $(DESTDIR)$(PREFIX)/share/man/man1
	install -m 644 docs/jail-ai.1 $(DESTDIR)$(PREFIX)/share/man/man1/
	gzip -f $(DESTDIR)$(PREFIX)/share/man/man1/jail-ai.1
	@echo "Man page installed successfully. Run 'man jail-ai' to view it."

uninstall-man: ## Uninstall man page
	@echo "Removing man page from $(DESTDIR)$(PREFIX)/share/man/man1/"
	rm -f $(DESTDIR)$(PREFIX)/share/man/man1/jail-ai.1.gz
	@echo "Man page uninstalled successfully."

view-man: docs/jail-ai.1 ## Preview the man page locally
	man ./docs/jail-ai.1
