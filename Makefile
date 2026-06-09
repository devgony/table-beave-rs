.PHONY: dev build test

# Start the local dev server with live reload.
dev:
	trunk serve

# Produce an optimized production build into dist/.
build:
	trunk build --release

# Run the parser test suite.
test:
	cargo test
