#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

echo "🚀 Starting Bare Rust Lambda Service"
echo "===================================="

# Check if Docker is running
print_status "Checking Docker status..."
if ! docker info >/dev/null 2>&1; then
    print_error "Docker is not running. Please start Docker and try again."
    exit 1
fi
print_success "Docker is running"

# Build the Rust service
print_status "Building Rust Lambda service..."
if cargo build --release --bin lambda-at-home-server; then
    print_success "Rust service built successfully"
else
    print_error "Failed to build Rust service"
    exit 1
fi

# Set environment variables
export RUST_LOG=info
export LAMBDA_RUNTIME_API_PORT=8080
export LAMBDA_RUNTIME_API_HOST=0.0.0.0

print_status "Starting Lambda Runtime API server..."
print_status "Server will be available at: http://localhost:8080"
print_status "Health check: http://localhost:8080/health"
print_status "Press Ctrl+C to stop the service"
echo ""

# Run the service
./target/release/lambda-at-home-server
