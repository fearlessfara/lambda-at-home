.PHONY: build test test-unit test-int fmt clippy clean run test-autoscaling test-service test-metrics test-node-runtimes test-apigw-proxy

# Build the project
build:
	cargo build

# Run unit tests only
test-unit:
	cargo test --workspace --lib

# Run integration tests (requires Docker and complete server implementation)
test-int:
	@echo "🐳 Starting Docker for integration tests..."
	@docker --version > /dev/null 2>&1 || (echo "❌ Docker is not installed or not running" && exit 1)
	@echo "⚠️  Integration tests require a complete Lambda@Home server implementation"
	@echo "⚠️  Currently, the server implementation is incomplete, so these tests will fail"
	@echo "🧪 Running integration tests..."
	cargo test --features docker_tests -- --ignored

# Run all tests
test: test-unit

# Format code
fmt:
	cargo fmt

# Check formatting
fmt-check:
	cargo fmt -- --check

# Run clippy
clippy:
	cargo clippy -- -D warnings

# Clean build artifacts
clean:
	cargo clean

# Run the server
run:
	cargo run --bin lambda-at-home-server

# Scripted smoke tests (require server + Docker)
test-autoscaling:
	./scripts/test-autoscaling.sh

test-service:
	./scripts/test-service.sh

test-metrics:
	./scripts/test-metrics.sh

# Node runtime test (18.x and 22.x)
test-node-runtimes:
	./scripts/test-node-runtimes.sh

# API Gateway path proxy test
test-apigw-proxy:
	./scripts/test-apigw-proxy.sh

# CI targets
ci: fmt-check clippy test-unit

# Full CI with integration tests (requires Docker)
ci-full: fmt-check clippy test-unit test-int

# Help
help:
	@echo "Available targets:"
	@echo "  build      - Build the project"
	@echo "  test       - Run unit tests"
	@echo "  test-unit  - Run unit tests only"
	@echo "  test-int   - Run integration tests (requires Docker)"
	@echo "  fmt        - Format code"
	@echo "  fmt-check  - Check code formatting"
	@echo "  clippy     - Run clippy linter"
	@echo "  clean      - Clean build artifacts"
	@echo "  run        - Run the server"
	@echo "  test-autoscaling - Run autoscaling burst + reuse smoke test"
	@echo "  test-service - Run end-to-end service smoke test"
	@echo "  test-metrics     - Check /metrics endpoint"
	@echo "  test-node-runtimes - Create & invoke functions with Node 18.x and 22.x"
	@echo "  test-apigw-proxy - Create function and invoke via /<function-name> path proxy"
	@echo "  ci         - Run CI checks (format, clippy, unit tests)"
	@echo "  ci-full    - Run full CI with integration tests"
	@echo "  help       - Show this help"
