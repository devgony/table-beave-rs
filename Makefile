.PHONY: dev build test deploy help

.DEFAULT_GOAL := help

dev: ## Start the local dev server with live reload
	trunk serve

build: ## Build an optimized production bundle into dist/
	trunk build --release

test: ## Run the parser test suite
	cargo test

deploy: build ## Build, then deploy to Cloudflare (Workers static assets)
	npx wrangler@latest deploy

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-8s\033[0m %s\n", $$1, $$2}'
