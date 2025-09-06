.PHONY: build test test-unit test-int fmt clippy clean run test-e2e test-service test-metrics test-node-runtimes ui-build release run-release

# Build the project
build: ui-build
	cd service && cargo build

# Build the web console (embedded into the binary at build time)
ui-build:
	cd console && npm ci && npm run build

# Run unit tests only
test-unit:
	cd service && cargo test --workspace --lib

# Test execution tracker module specifically
test-execution-tracker:
	cd service && cargo test --package lambda-control --lib

# Test refactored registry functionality
test-registry:
	cd service && cargo test --package lambda-control --lib

# Run integration tests (requires Docker and complete server implementation)
test-int:
	@echo "🐳 Starting Docker for integration tests..."
	@docker --version > /dev/null 2>&1 || (echo "❌ Docker is not installed or not running" && exit 1)
	@echo "⚠️  Integration tests require a complete Lambda@Home server implementation"
	@echo "⚠️  Currently, the server implementation is incomplete, so these tests will fail"
	@echo "🧪 Running integration tests..."
	cd service && cargo test --features docker_tests -- --ignored

# Run all tests
test: test-unit

# Format code
fmt:
	cd service && cargo fmt

# Check formatting
fmt-check:
	cd service && cargo fmt -- --check

# Run clippy
clippy:
	cd service && cargo clippy -- -D warnings

# Clean build artifacts
clean:
	cd service && cargo clean

# Run the server
run:
	cd service && cargo run --bin lambda-at-home-server

# Build a release binary with embedded console assets
release: ui-build
	cd service && cargo build --release --bin lambda-at-home-server

# Run the release binary
run-release:
	cd service && ./target/release/lambda-at-home-server

# E2E tests (require server + Docker)
test-e2e:
	cd e2e && npm test

test-service:
	cd e2e && npm run test:service

test-metrics:
	cd e2e && npm run test:metrics

# Node runtime test (18.x and 22.x)
test-node-runtimes:
	cd e2e && npm run test:runtimes

# CI targets
ci: fmt-check clippy test-unit test-execution-tracker test-registry

# Full CI with integration tests (requires Docker)
ci-full: fmt-check clippy test-unit test-execution-tracker test-registry test-int

# Help
help:
	@echo "Available targets:"
	@echo "  build      - Build the project"
	@echo "  ui-build   - Build the web console (embedded assets)"
	@echo "  release    - Build release binary with embedded console"
	@echo "  run-release- Run the release binary"
	@echo "  test       - Run unit tests"
	@echo "  test-unit  - Run unit tests only"
	@echo "  test-execution-tracker - Test execution tracker module"
	@echo "  test-registry - Test refactored registry functionality"
	@echo "  test-int   - Run integration tests (requires Docker)"
	@echo "  fmt        - Format code"
	@echo "  fmt-check  - Check code formatting"
	@echo "  clippy     - Run clippy linter"
	@echo "  clean      - Clean build artifacts"
	@echo "  run        - Run the server"
	@echo "  test-e2e - Run all e2e tests"
	@echo "  test-service - Run end-to-end service smoke test"
	@echo "  test-metrics     - Check /metrics endpoint"
	@echo "  test-node-runtimes - Create & invoke functions with Node 18.x and 22.x"
	@echo "  ci         - Run CI checks (format, clippy, unit tests)"
	@echo "  ci-full    - Run full CI with integration tests"
	@echo "  help       - Show this help"
