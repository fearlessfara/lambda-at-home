# Makefile for building Lambda runtime images

.PHONY: build-runtimes clean-runtimes test

# Build all runtime images
build-runtimes: javascript-runtime python-runtime

# Build JavaScript runtime
javascript-runtime:
	@echo "Building JavaScript runtime image..."
	docker build -t javascript-runtime -f runtimes/nodejs/Dockerfile .
	@echo "JavaScript runtime image built successfully"

# Build Python runtime
python-runtime:
	@echo "Building Python runtime image..."
	docker build -t python-runtime -f runtimes/python/Dockerfile .
	@echo "Python runtime image built successfully"

# Clean runtime images
clean-runtimes:
	@echo "Cleaning runtime images..."
	docker rmi javascript-runtime python-runtime 2>/dev/null || true
	@echo "Runtime images cleaned"

# Run tests
test: build-runtimes
	@echo "Running tests..."
	cargo test --test api_integration_tests -- --nocapture

# Help
help:
	@echo "Available targets:"
	@echo "  build-runtimes  - Build all runtime images"
	@echo "  javascript-runtime - Build JavaScript runtime image"
	@echo "  python-runtime  - Build Python runtime image"
	@echo "  clean-runtimes  - Remove runtime images"
	@echo "  test           - Build runtimes and run tests"
	@echo "  help           - Show this help"