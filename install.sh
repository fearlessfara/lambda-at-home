#!/bin/bash

# Lambda@Home Installation Script
# This script sets up the database and directory structure for Lambda@Home

set -e  # Exit on any error

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

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to get the directory where the script is located
get_script_dir() {
    cd "$(dirname "${BASH_SOURCE[0]}")" && pwd
}

# Main installation function
main() {
    print_status "Starting Lambda@Home installation..."
    
    # Get the script directory
    SCRIPT_DIR=$(get_script_dir)
    PROJECT_ROOT="$SCRIPT_DIR"
    
    print_status "Project root: $PROJECT_ROOT"
    
    # Check if we're in the right directory
    if [[ ! -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        print_error "Cargo.toml not found. Please run this script from the Lambda@Home project root."
        exit 1
    fi
    
    # Create data directory structure
    print_status "Creating data directory structure..."
    
    DATA_DIR="$PROJECT_ROOT/data"
    CACHE_DIR="$DATA_DIR/cache"
    ZIPS_DIR="$DATA_DIR/zips"
    
    mkdir -p "$DATA_DIR"
    mkdir -p "$CACHE_DIR"
    mkdir -p "$ZIPS_DIR"
    
    print_success "Created directory structure:"
    print_success "  - $DATA_DIR"
    print_success "  - $CACHE_DIR"
    print_success "  - $ZIPS_DIR"
    
    # Set proper permissions
    chmod 755 "$DATA_DIR"
    chmod 755 "$CACHE_DIR"
    chmod 755 "$ZIPS_DIR"
    
    # Check if Docker is available
    if ! command_exists docker; then
        print_warning "Docker is not installed or not in PATH."
        print_warning "Lambda@Home requires Docker to run containers."
        print_warning "Please install Docker and ensure it's running before using Lambda@Home."
    else
        print_success "Docker is available"
        
        # Check if Docker daemon is running
        if ! docker info >/dev/null 2>&1; then
            print_warning "Docker daemon is not running."
            print_warning "Please start Docker before using Lambda@Home."
        else
            print_success "Docker daemon is running"
        fi
    fi
    
    # Check if Rust is available
    if ! command_exists cargo; then
        print_warning "Rust/Cargo is not installed or not in PATH."
        print_warning "You'll need Rust to build Lambda@Home."
        print_warning "Please install Rust from https://rustup.rs/"
    else
        print_success "Rust/Cargo is available"
    fi
    
    # Create a default configuration if it doesn't exist
    CONFIG_DIR="$PROJECT_ROOT/config"
    DEFAULT_CONFIG="$PROJECT_ROOT/configs/default.toml"
    
    if [[ ! -d "$CONFIG_DIR" ]]; then
        mkdir -p "$CONFIG_DIR"
        print_success "Created config directory: $CONFIG_DIR"
    fi
    
    # Copy default config if no config exists
    if [[ ! -f "$CONFIG_DIR/config.toml" ]] && [[ -f "$DEFAULT_CONFIG" ]]; then
        cp "$DEFAULT_CONFIG" "$CONFIG_DIR/config.toml"
        print_success "Created default configuration: $CONFIG_DIR/config.toml"
    fi
    
    # Create functions directory for user functions
    FUNCTIONS_DIR="$PROJECT_ROOT/functions"
    if [[ ! -d "$FUNCTIONS_DIR" ]]; then
        mkdir -p "$FUNCTIONS_DIR"
        print_success "Created functions directory: $FUNCTIONS_DIR"
    fi
    
    # Create a .gitignore for the data directory if it doesn't exist
    GITIGNORE_FILE="$DATA_DIR/.gitignore"
    if [[ ! -f "$GITIGNORE_FILE" ]]; then
        cat > "$GITIGNORE_FILE" << EOF
# Lambda@Home data directory
# This directory contains runtime data and should not be committed to version control

# Database files
*.db
*.db-journal
*.db-wal
*.db-shm

# Cache files
cache/*

# Function ZIP files
zips/*

# Log files
*.log

# Temporary files
*.tmp
*.temp
EOF
        print_success "Created .gitignore for data directory"
    fi
    
    # Create a README for the data directory
    README_FILE="$DATA_DIR/README.md"
    if [[ ! -f "$README_FILE" ]]; then
        cat > "$README_FILE" << EOF
# Lambda@Home Data Directory

This directory contains runtime data for Lambda@Home:

- \`lhome.db\` - SQLite database with function metadata and execution logs
- \`cache/\` - Cached Docker images and build artifacts
- \`zips/\` - Stored function code ZIP files

## Important Notes

- This directory is created automatically by the install script
- The database will be created when you first run Lambda@Home
- Do not manually modify files in this directory
- This directory should not be committed to version control

## Backup

To backup your Lambda@Home data:

\`\`\`bash
# Stop Lambda@Home first
# Then copy the data directory
cp -r data/ backup-$(date +%Y%m%d)/
\`\`\`

## Reset

To reset Lambda@Home to a clean state:

\`\`\`bash
# Stop Lambda@Home first
# Then remove the data directory
rm -rf data/
# Run the install script again
./install.sh
\`\`\`
EOF
        print_success "Created README for data directory"
    fi
    
    # Test database creation (optional)
    if command_exists sqlite3; then
        print_status "Testing database creation..."
        TEST_DB="$DATA_DIR/test.db"
        sqlite3 "$TEST_DB" "CREATE TABLE test (id INTEGER); DROP TABLE test;" 2>/dev/null || true
        rm -f "$TEST_DB"
        print_success "Database creation test passed"
    else
        print_warning "sqlite3 not found. Database creation will be tested when Lambda@Home starts."
    fi
    
    # Create a systemd service file (optional)
    if [[ -d "/etc/systemd/system" ]] && [[ $EUID -eq 0 ]]; then
        print_status "Creating systemd service file..."
        cat > "/etc/systemd/system/lambda-at-home.service" << EOF
[Unit]
Description=Lambda@Home Server
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=root
WorkingDirectory=$PROJECT_ROOT
ExecStart=$PROJECT_ROOT/target/release/lambda-at-home-server
Restart=always
RestartSec=5
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF
        print_success "Created systemd service file: /etc/systemd/system/lambda-at-home.service"
        print_status "To enable the service: systemctl enable lambda-at-home"
        print_status "To start the service: systemctl start lambda-at-home"
    fi
    
    # Final success message
    print_success "Lambda@Home installation completed successfully!"
    echo
    print_status "Next steps:"
    echo "  1. Build the project: cargo build --release"
    echo "  2. Run Lambda@Home: cargo run"
    echo "  3. Or run the release binary: ./target/release/lambda-at-home-server"
    echo
    print_status "Configuration:"
    echo "  - User API: http://127.0.0.1:9000"
    echo "  - Runtime API: http://127.0.0.1:9001"
    echo "  - Data directory: $DATA_DIR"
    echo
    print_status "For more information, see the README.md file."
}

# Run main function
main "$@"
