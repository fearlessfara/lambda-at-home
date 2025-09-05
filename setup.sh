#!/bin/bash

# Lambda@Home Quick Setup Script
# This script sets up Lambda@Home for end users

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

main() {
    print_status "Lambda@Home Quick Setup"
    echo
    
    # Check prerequisites
    print_status "Checking prerequisites..."
    
    if ! command_exists docker; then
        print_error "Docker is required but not installed."
        print_error "Please install Docker from https://docs.docker.com/get-docker/"
        exit 1
    fi
    
    if ! docker info >/dev/null 2>&1; then
        print_error "Docker daemon is not running."
        print_error "Please start Docker and try again."
        exit 1
    fi
    
    if ! command_exists cargo; then
        print_error "Rust/Cargo is required but not installed."
        print_error "Please install Rust from https://rustup.rs/"
        exit 1
    fi
    
    print_success "All prerequisites are met!"
    
    # Get current directory
    CURRENT_DIR=$(pwd)
    print_status "Setting up Lambda@Home in: $CURRENT_DIR"
    
    # Create necessary directories
    print_status "Creating directory structure..."
    
    mkdir -p data/cache
    mkdir -p data/zips
    mkdir -p config
    mkdir -p functions
    
    print_success "Created directory structure"
    
    # Create default config if it doesn't exist
    if [[ ! -f "config/config.toml" ]]; then
        cat > config/config.toml << 'EOF'
[server]
bind = "127.0.0.1"
port_user_api = 9000
port_runtime_api = 9001

[data]
dir = "data"
db_url = "sqlite://data/lhome.db"

[docker]
host = ""

[defaults]
memory_mb = 512
timeout_ms = 3000
tmp_mb = 512

[idle]
soft_ms = 45000   # stop container
hard_ms = 300000  # rm container

[limits]
max_global_concurrency = 256
EOF
        print_success "Created default configuration"
    fi
    
    # Create .gitignore for data directory
    cat > data/.gitignore << 'EOF'
# Lambda@Home data directory
*.db
*.db-journal
*.db-wal
*.db-shm
cache/*
zips/*
*.log
*.tmp
*.temp
EOF
    
    print_success "Created .gitignore for data directory"
    
    # Set permissions
    chmod 755 data
    chmod 755 data/cache
    chmod 755 data/zips
    chmod 755 config
    chmod 755 functions
    
    print_success "Set directory permissions"
    
    # Final instructions
    echo
    print_success "Lambda@Home setup completed!"
    echo
    print_status "Next steps:"
    echo "  1. Build the project: cargo build --release"
    echo "  2. Run Lambda@Home: cargo run"
    echo "  3. Or run the release binary: ./target/release/lambda-at-home-server"
    echo
    print_status "Lambda@Home will be available at:"
    echo "  - User API: http://127.0.0.1:9000"
    echo "  - Runtime API: http://127.0.0.1:9001"
    echo
    print_status "The database will be created automatically on first run."
    echo
    print_warning "Make sure Docker is running before starting Lambda@Home!"
}

main "$@"
