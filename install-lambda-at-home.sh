#!/bin/bash

# Lambda@Home Installation Script
# Downloads and installs the latest Lambda@Home binary for your platform

set -e

# Check if we're running in a compatible shell
if [[ -z "$BASH_VERSION" ]]; then
    echo "Error: This script requires bash. Please run it with:"
    echo "  bash <(curl -fsSL https://raw.githubusercontent.com/fearlessfara/lambda-at-home/main/install-lambda-at-home.sh)"
    exit 1
fi

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
        Darwin*)    os="macos" ;;
        CYGWIN*|MINGW*|MSYS*) os="windows" ;;
        *)          print_error "Unsupported operating system: $(uname -s)"; exit 1 ;;
    esac
    
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        arm64|aarch64) arch="arm64" ;;
        armv7l) arch="armv7" ;;
        *) print_error "Unsupported architecture: $(uname -m)"; exit 1 ;;
    esac
    
    echo "${os}-${arch}"
}

# Function to download and verify binary
download_binary() {
    local version=$1
    local platform=$2
    local binary_name="lambda-at-home-server"
    local download_url="https://github.com/fearlessfara/lambda-at-home/releases/download/${version}/lambda-at-home-server-${version}-${platform}"
    
    # Add .exe extension for Windows
    if [[ "$platform" == *"windows"* ]]; then
        download_url="${download_url}.exe"
        binary_name="lambda-at-home-server.exe"
    fi
    
    print_status "Downloading Lambda@Home ${version} for ${platform}..."
    print_status "URL: ${download_url}"
    
    # Try to download the binary
    local download_success=false
    if command_exists curl; then
        if curl -L -o "${binary_name}" "${download_url}"; then
            download_success=true
        fi
    elif command_exists wget; then
        if wget -O "${binary_name}" "${download_url}"; then
            download_success=true
        fi
    fi
    
    # If download failed, try fallback to x86_64 for macOS ARM64 (Rosetta compatibility)
    if [[ "$download_success" == false ]] && [[ "$platform" == "macos-arm64" ]]; then
        print_warning "ARM64 binary not available, trying x86_64 fallback for macOS (Rosetta compatibility)..."
        local fallback_platform="macos-x86_64"
        local fallback_url="https://github.com/fearlessfara/lambda-at-home/releases/download/${version}/lambda-at-home-server-${version}-${fallback_platform}"
        
        print_status "Trying fallback URL: ${fallback_url}"
        
        if command_exists curl; then
            if curl -L -o "${binary_name}" "${fallback_url}"; then
                download_success=true
                print_success "Downloaded x86_64 binary (will run via Rosetta on Apple Silicon)"
            fi
        elif command_exists wget; then
            if wget -O "${binary_name}" "${fallback_url}"; then
                download_success=true
                print_success "Downloaded x86_64 binary (will run via Rosetta on Apple Silicon)"
            fi
        fi
    fi
    
    if [[ "$download_success" == false ]]; then
        print_error "Failed to download binary"
        print_error "Tried URL: ${download_url}"
        if [[ "$platform" == "macos-arm64" ]]; then
            print_error "Also tried x86_64 fallback for macOS (Rosetta compatibility)"
        fi
        exit 1
    fi
    
    print_success "Binary downloaded successfully: ${binary_name}"
}

# Function to verify binary checksum
verify_checksum() {
    local version=$1
    local platform=$2
    local binary_name="lambda-at-home-server"
    local checksum_url="https://github.com/fearlessfara/lambda-at-home/releases/download/${version}/lambda-at-home-server-${version}-${platform}.sha256"
    
    # Add .exe extension for Windows
    if [[ "$platform" == *"windows"* ]]; then
        checksum_url="${checksum_url}.exe.sha256"
        binary_name="lambda-at-home-server.exe"
    fi
    
    print_status "Verifying binary checksum..."
    
    if command_exists curl; then
        curl -L -o "checksum.sha256" "${checksum_url}"
    elif command_exists wget; then
        wget -O "checksum.sha256" "${checksum_url}"
    fi
    
    if [[ -f "checksum.sha256" ]]; then
        # Create a temporary checksum file with the correct binary name
        # The downloaded checksum file has the full filename, but our binary has a simple name
        local full_binary_name="lambda-at-home-server-${version}-${platform}"
        if [[ "$platform" == *"windows"* ]]; then
            full_binary_name="${full_binary_name}.exe"
        fi
        
        # Replace the full filename in checksum with our simple binary name
        sed "s/${full_binary_name}/${binary_name}/" checksum.sha256 > checksum_fixed.sha256
        
        if command_exists sha256sum; then
            if sha256sum -c checksum_fixed.sha256; then
                print_success "Checksum verification passed"
            else
                print_error "Checksum verification failed"
                exit 1
            fi
        elif command_exists shasum; then
            if shasum -a 256 -c checksum_fixed.sha256; then
                print_success "Checksum verification passed"
            else
                print_error "Checksum verification failed"
                exit 1
            fi
        else
            print_warning "No checksum verification tool found, skipping verification"
        fi
        rm -f checksum.sha256 checksum_fixed.sha256
    else
        print_warning "Could not download checksum file, skipping verification"
    fi
}

# Function to check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
    # Check if we're on Windows
    if [[ "$(uname -s)" == *"MINGW"* ]] || [[ "$(uname -s)" == *"CYGWIN"* ]] || [[ "$(uname -s)" == *"MSYS"* ]]; then
        print_warning "Windows detected. This script requires:"
        print_warning "1. Git Bash, WSL, or MSYS2"
        print_warning "2. Docker Desktop for Windows"
        print_warning "3. Make sure Docker Desktop is running"
        echo
    fi
    
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

# Function to setup local lambda@home directory
setup_local_directory() {
    local lambda_dir="lambda@home"
    
    print_status "Setting up local Lambda@Home directory..."
    
    # Create lambda@home directory
    mkdir -p "$lambda_dir"
    cd "$lambda_dir"
    
    # Handle Windows path issues
    if [[ "$(uname -s)" == *"MINGW"* ]] || [[ "$(uname -s)" == *"CYGWIN"* ]] || [[ "$(uname -s)" == *"MSYS"* ]]; then
        print_status "Windows environment detected - using Windows-compatible paths"
    fi
    
    # Create data directory structure
    mkdir -p "data/cache"
    mkdir -p "data/zips"
    mkdir -p "config"
    mkdir -p "functions"
    
    # Create the database file
    touch "data/lhome.db"
    print_success "Created database file at data/lhome.db"
    
    # Create default config
    cat > "config/config.toml" << 'EOF'
[server]
bind = "127.0.0.1"
port_user_api = 9000
port_runtime_api = 9001
max_request_body_size_mb = 50

[data]
dir = "data"
db_url = "sqlite:data/lhome.db"

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
    print_success "Created default configuration at config/config.toml"
    
    # Create .gitignore for data directory
    cat > "data/.gitignore" << 'EOF'
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
    
    print_success "Local Lambda@Home directory created at ./$lambda_dir"
    print_status "Directory structure:"
    echo "  ./$lambda_dir/"
    echo "  ├── lambda-at-home-server* (binary will be here)"
    echo "  ├── data/"
    echo "  │   ├── lhome.db"
    echo "  │   ├── cache/"
    echo "  │   └── zips/"
    echo "  ├── config/"
    echo "  │   └── config.toml"
    echo "  └── functions/"
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
    
    # Setup local lambda@home directory
    setup_local_directory
    
    # Download binary to the lambda@home directory
    download_binary "$version" "$platform"
    verify_checksum "$version" "$platform"
    
    # Make binary executable
    chmod +x lambda-at-home-server*
    
    # Final success message
    echo
    print_success "Lambda@Home setup completed successfully!"
    echo
    print_status "Next steps:"
    echo "  1. Start Lambda@Home:"
    echo "     cd lambda@home"
    echo "     ./lambda-at-home-server*"
    echo "  2. Or run with custom config:"
    echo "     ./lambda-at-home-server* --config config/config.toml"
    echo
    print_status "Lambda@Home will be available at:"
    echo "  - User API: http://127.0.0.1:9000"
    echo "  - Web Console: http://127.0.0.1:9000"
    echo "  - Runtime API: http://127.0.0.1:9001"
    echo
    print_status "Directory structure created:"
    echo "  ./lambda@home/"
    echo "  ├── lambda-at-home-server* (your binary)"
    echo "  ├── data/"
    echo "  │   ├── lhome.db (database file)"
    echo "  │   ├── cache/"
    echo "  │   └── zips/"
    echo "  ├── config/"
    echo "  │   └── config.toml"
    echo "  └── functions/"
    echo
    print_warning "Make sure Docker is running before starting Lambda@Home!"
}

# Run main function
main "$@"