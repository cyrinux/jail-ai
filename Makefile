.PHONY: help build-image push-image test-image clean build run test clippy fmt

IMAGE_NAME ?= localhost/jail-ai-env
IMAGE_TAG ?= latest
IMAGE_FULL = $(IMAGE_NAME):$(IMAGE_TAG)

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build-image: ## Build the AI agent environment container image
	@echo "Building container image: $(IMAGE_FULL)"
	podman build -t $(IMAGE_FULL) -f Containerfile .
	@echo "Image built successfully: $(IMAGE_FULL)"

push-image: ## Push the container image to a registry
	@echo "Pushing image: $(IMAGE_FULL)"
	podman push $(IMAGE_FULL)

test-image: ## Test the container image
	@echo "Testing image: $(IMAGE_FULL)"
	@podman run --rm $(IMAGE_FULL) bash --version
	@podman run --rm $(IMAGE_FULL) zsh --version
	@podman run --rm $(IMAGE_FULL) fzf --version
	@podman run --rm $(IMAGE_FULL) rg --version
	@podman run --rm $(IMAGE_FULL) cargo --version
	@podman run --rm $(IMAGE_FULL) go version
	@podman run --rm $(IMAGE_FULL) node --version
	@podman run --rm $(IMAGE_FULL) python --version
	@podman run --rm $(IMAGE_FULL) claude --version
	@podman run --rm $(IMAGE_FULL) copilot --version || echo "Copilot CLI installed (requires auth)"
	@podman run --rm $(IMAGE_FULL) cursor-agent --version || echo "Cursor Agent installed (requires auth)"
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

dev-jail: build-image ## Create a development jail with the custom image
	cargo run -- create dev-agent --backend podman --image $(IMAGE_FULL) --no-network

# Example usage targets
.PHONY: example-create example-exec example-remove

example-create: build-image ## Example: Create a jail with custom image
	cargo run -- create my-agent --backend podman --image $(IMAGE_FULL)

example-exec: ## Example: Execute a command in the jail
	cargo run -- exec my-agent -- ls -la /workspace

example-remove: ## Example: Remove the jail
	cargo run -- remove my-agent --force
