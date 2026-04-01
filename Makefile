.PHONY: help all build install build-ebpf build-loader install-loader build-all clean-ebpf run test clippy fmt completions bottle-macos

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

update-cloud-versions: ## Update cloud provider tool versions to latest
	@echo "Updating cloud provider tool versions..."
	./scripts/update-cloud-versions.sh
	@echo ""
	@echo "Review changes with: git diff containerfiles/"
	@echo "Rebuild with: make rebuild-cloud-layers"

rebuild-cloud-layers: ## Force rebuild cloud layers with updated versions
	@echo "Rebuilding cloud layers..."
	cargo run -- claude --cloud --upgrade --force-layers aws,gcp --verbose

completions: build ## Generate shell completions into dist/completions/
	@mkdir -p dist/completions
	./target/release/jail-ai completions bash > dist/completions/jail-ai.bash
	./target/release/jail-ai completions zsh  > dist/completions/_jail-ai
	./target/release/jail-ai completions fish > dist/completions/jail-ai.fish
	@echo "✓ Completions written to dist/completions/"

bottle-macos: build completions ## Build a local Homebrew bottle for the current macOS host
	@VERSION=$$(cargo metadata --no-deps --format-version 1 | python3 -c 'import sys,json; print(json.load(sys.stdin)["packages"][0]["version"])');\
	OS_TAG=$$(sw_vers -productName 2>/dev/null | tr '[:upper:]' '[:lower:]' | tr ' ' '_');\
	BOTTLE_DIR="jail-ai/$${VERSION}/bin";\
	mkdir -p "$${BOTTLE_DIR}";\
	cp target/release/jail-ai "$${BOTTLE_DIR}/";\
	mkdir -p "jail-ai/$${VERSION}/completions";\
	cp dist/completions/* "jail-ai/$${VERSION}/completions/";\
	[ -f docs/jail-ai.1 ] && cp docs/jail-ai.1 "jail-ai/$${VERSION}/" || true;\
	tar -czf "jail-ai-$${OS_TAG}.tar.gz" jail-ai/;\
	sha256sum "jail-ai-$${OS_TAG}.tar.gz" 2>/dev/null || shasum -a 256 "jail-ai-$${OS_TAG}.tar.gz";\
	rm -rf jail-ai/;\
	echo "✓ Bottle: jail-ai-$${OS_TAG}.tar.gz"
