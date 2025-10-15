.PHONY: help all build install build-ebpf build-loader install-loader build-all clean-ebpf run test clippy fmt

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

all: install build-ebpf install-loader ## Full install: build and install everything (main binary, eBPF programs, loader)

build: ## Build the jail-ai binary (release)
	cargo build --release

install: build ## Build and install the jail-ai binary
	cargo install --path .

build-ebpf: ## Build eBPF programs in a container (reuses container if exists)
	@echo "Building eBPF programs..."
	./build-ebpf.sh

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
