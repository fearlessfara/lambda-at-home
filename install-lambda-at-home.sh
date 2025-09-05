#!/bin/bash

# Lambda@Home Installation Script
# Downloads and installs the latest Lambda@Home binary for your platform

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

# Function to get the latest release version
get_latest_version() {
    if command_exists curl; then
        curl -s https://api.github.com/repos/fearlessfara/lambda-at-home/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    elif command_exists wget; then
        wget -qO- https://api.github.com/repos/fearlessfara/lambda-at-home/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
    else
        print_error "curl or wget is required to download the binary"
        exit 1
    fi
}

# Function to detect OS and architecture
detect_platform() {
    local os arch
    
    case "$(uname -s)" in
        Linux*)     os="linux" ;;
        Darwin*)    os="darwin" ;;
        CYGWIN*|MINGW*|MSYS*) os="windows" ;;
        *)          print_error "Unsupported operating system: $(uname -s)"; exit 1 ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="aarch64" ;;
        armv7l) arch="armv7" ;;
        *) print_error "Unsupported architecture: $(uname -m)"; exit 1 ;;
    esac
    
    echo "${os}-${arch}"
}

# Function to download and verify binary
download_binary() {
    local version=$1
    local platform=$2
    local download_url="https://github.com/fearlessfara/lambda-at-home/releases/download/${version}/lambda-at-home-server-${version}-${platform}"
    
    # Add .exe extension for Windows
    if [[ "$platform" == *"windows"* ]]; then
        download_url="${download_url}.exe"
    fi
    
    print_status "Downloading Lambda@Home ${version} for ${platform}..."
    print_status "URL: ${download_url}"
    
    if command_exists curl; then
        curl -L -o "lambda-at-home-server" "${download_url}"
    elif command_exists wget; then
        wget -O "lambda-at-home-server" "${download_url}"
    fi
    
    if [[ ! -f "lambda-at-home-server" ]]; then
        print_error "Failed to download binary"
        exit 1
    fi
    
    print_success "Binary downloaded successfully"
}

# Function to verify binary checksum
verify_checksum() {
    local version=$1
    local platform=$2
    local checksum_url="https://github.com/fearlessfara/lambda-at-home/releases/download/${version}/lambda-at-home-server-${version}-${platform}.sha256"
    
    # Add .exe extension for Windows
    if [[ "$platform" == *"windows"* ]]; then
        checksum_url="${checksum_url}.exe.sha256"
    fi
    
    print_status "Verifying binary checksum..."
    
    if command_exists curl; then
        curl -L -o "checksum.sha256" "${checksum_url}"
    elif command_exists wget; then
        wget -O "checksum.sha256" "${checksum_url}"
    fi
    
    if [[ -f "checksum.sha256" ]]; then
        if command_exists sha256sum; then
            sha256sum -c checksum.sha256
        elif command_exists shasum; then
            shasum -a 256 -c checksum.sha256
        else
            print_warning "No checksum verification tool found, skipping verification"
        fi
        rm -f checksum.sha256
    else
        print_warning "Could not download checksum file, skipping verification"
    fi
}

# Function to install binary
install_binary() {
    local install_dir="/usr/local/bin"
    local binary_name="lambda-at-home-server"
    
    # Check if we have write permissions to /usr/local/bin
    if [[ ! -w "$install_dir" ]]; then
        print_status "Installing to ~/.local/bin (no write permission to $install_dir)"
        install_dir="$HOME/.local/bin"
        mkdir -p "$install_dir"
    fi
    
    print_status "Installing binary to $install_dir..."
    chmod +x "lambda-at-home-server"
    mv "lambda-at-home-server" "$install_dir/$binary_name"
    
    print_success "Binary installed to $install_dir/$binary_name"
    
    # Add to PATH if needed
    if [[ "$install_dir" == "$HOME/.local/bin" ]]; then
        if ! echo "$PATH" | grep -q "$install_dir"; then
            print_warning "Please add $install_dir to your PATH:"
            print_warning "  export PATH=\"\$PATH:$install_dir\""
            print_warning "  Add this line to your ~/.bashrc, ~/.zshrc, or ~/.profile"
        fi
    fi
}

# Function to create data directory and config
setup_data_directory() {
    local data_dir="$HOME/.lambda-at-home"
    
    print_status "Setting up data directory..."
    
    mkdir -p "$data_dir/data/cache"
    mkdir -p "$data_dir/data/zips"
    mkdir -p "$data_dir/config"
    mkdir -p "$data_dir/functions"
    
    # Create default config if it doesn't exist
    if [[ ! -f "$data_dir/config/config.toml" ]]; then
        cat > "$data_dir/config/config.toml" << 'EOF'
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
        print_success "Created default configuration at $data_dir/config/config.toml"
    fi
    
    # Create .gitignore for data directory
    cat > "$data_dir/data/.gitignore" << 'EOF'
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
    
    print_success "Data directory created at $data_dir"
}

# Function to check prerequisites
check_prerequisites() {
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
    
    print_success "All prerequisites are met!"
}

# Function to create systemd service (optional)
create_systemd_service() {
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
WorkingDirectory=$HOME/.lambda-at-home
ExecStart=/usr/local/bin/lambda-at-home-server
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
}

# Main installation function
main() {
    print_status "Lambda@Home Installation Script"
    echo
    
    # Check prerequisites
    check_prerequisites
    
    # Get latest version
    print_status "Fetching latest version..."
    local version
    version=$(get_latest_version)
    if [[ -z "$version" ]]; then
        print_error "Failed to get latest version"
        exit 1
    fi
    print_success "Latest version: $version"
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    print_success "Detected platform: $platform"
    
    # Create temporary directory
    local temp_dir
    temp_dir=$(mktemp -d)
    cd "$temp_dir"
    
    # Download and install
    download_binary "$version" "$platform"
    verify_checksum "$version" "$platform"
    install_binary
    
    # Setup data directory
    setup_data_directory
    
    # Create systemd service (if running as root)
    create_systemd_service
    
    # Cleanup
    cd /
    rm -rf "$temp_dir"
    
    # Final success message
    echo
    print_success "Lambda@Home installation completed successfully!"
    echo
    print_status "Next steps:"
    echo "  1. Start Lambda@Home: lambda-at-home-server"
    echo "  2. Or run with custom config: lambda-at-home-server --config ~/.lambda-at-home/config/config.toml"
    echo
    print_status "Lambda@Home will be available at:"
    echo "  - User API: http://127.0.0.1:9000"
    echo "  - Web Console: http://127.0.0.1:9000 (embedded in binary)"
    echo "  - Runtime API: http://127.0.0.1:9001"
    echo
    print_status "Configuration and data are stored in: ~/.lambda-at-home/"
    echo
    print_warning "Make sure Docker is running before starting Lambda@Home!"
}

# Run main function
main "$@"
