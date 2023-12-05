.PHONY: run build lint clean clean

build-service: ## Build notification service
	cargo +nightly build --release --manifest-path ./notification-service/Cargo.toml

build-contracts: ## Build smart contracts
	cargo +nightly contract build --release --manifest-path ./contracts/subscriptions/Cargo.toml

build: build-service build-contracts ## build all

lint: ## Run the linter
	cargo +nightly fmt
	cargo +nightly clippy --release -- -D warnings

test-contracts: ## Run unit tests for smart contracts
	cargo test --manifest-path ./contracts/subscriptions/Cargo.toml

clean: clean-service clean-contracts ## Clean all temporary files

clean-service: ## Clean all temporary files for the notification service
	cargo clean --manifest-path ./notification-service/Cargo.toml
	
clean-contracts: ## Clean all temporary files for smart contracts
	cargo clean --manifest-path ./contracts/subscriptions/Cargo.toml

help: ## Displays this help
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make \033[1;36m<target>\033[0m\n\nTargets:\n"} /^[a-zA-Z0-9_-]+:.*?##/ { printf "  \033[1;36m%-25s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)
