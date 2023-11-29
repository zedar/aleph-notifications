.PHONY: run build lint clean clean

build: ## Build all
	cargo +nightly build --all --release

lint: ## Run the linter
	cargo +nightly fmt
	cargo +nightly clippy --release -- -D warnings

test: ## Run unit tests
	cargo test

clean: ## Clean all temporary files
	cargo clean

help: ## Displays this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[1;36m<target>\033[0m\n\nTargets:\n"} /^[a-zA-Z0-9_-]+:.*?##/ { printf "  \033[1;36m%-25s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
