.PHONY: help build-image push-image test-image clean build run test clippy fmt install-man uninstall-man view-man

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
	@echo "Note: jail-ai will automatically build the default image when needed."
	@echo "This target is useful for testing image builds before they are needed."
	podman build -t $(IMAGE_FULL) -f Containerfile \
		--build-arg PUID=$$(id -u) \
		--build-arg PGID=$$(id -g) \
		.
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
	@echo "All tools verified successfully!"

clean: ## Remove built container images
	@echo "Removing image: $(IMAGE_FULL)"
	-podman rmi $(IMAGE_FULL)

build: ## Build the jail-ai binary
	cargo build --release

run: ## Run jail-ai (use ARGS="..." to pass arguments)
	cargo run -- $(ARGS)

test: ## Run tests
	cargo test

clippy: ## Run clippy lints
	cargo clippy -- -D warnings

fmt: ## Format code
	cargo fmt

dev-jail: ## Create a development jail (image auto-built if needed)
	cargo run -- create dev-agent --backend podman --image $(IMAGE_FULL) --no-network

# Example usage targets
.PHONY: example-create example-exec example-remove

example-create: ## Example: Create a jail (image auto-built if needed)
	cargo run -- create my-agent --backend podman --image $(IMAGE_FULL)

example-exec: ## Example: Execute a command in the jail
	cargo run -- exec my-agent -- ls -la /workspace

example-remove: ## Example: Remove the jail
	cargo run -- remove my-agent --force

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
