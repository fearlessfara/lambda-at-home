# Use Ubuntu as base image (glibc-based for compatibility with pre-built binaries)
FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    docker.io \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Create data directory for the service
RUN mkdir -p /app/data

# Set up environment variables
ENV RUST_LOG=info
ENV LAMBDA_AT_HOME_DATA_DIR=/app/data

# Download and install the latest release
RUN ARCH=$(uname -m) && \
    if [ "$ARCH" = "x86_64" ]; then PLATFORM="linux-x86_64"; \
    elif [ "$ARCH" = "aarch64" ]; then PLATFORM="linux-arm64"; \
    else echo "Unsupported architecture: $ARCH" && exit 1; fi && \
    echo "Detected platform: $PLATFORM" && \
    LATEST_TAG=$(curl -s https://api.github.com/repos/fearlessfara/lambda-at-home/releases/latest | grep '"tag_name":' | head -1 | sed -E 's/.*"([^"]+)".*/\1/') && \
    echo "Latest release: $LATEST_TAG" && \
    DOWNLOAD_URL="https://github.com/fearlessfara/lambda-at-home/releases/download/${LATEST_TAG}/lambda-at-home-server-${LATEST_TAG}-${PLATFORM}" && \
    echo "Downloading from: $DOWNLOAD_URL" && \
    curl -L -o /app/lambda-at-home-server "$DOWNLOAD_URL" && \
    chmod +x /app/lambda-at-home-server && \
    echo "Binary downloaded and made executable"

# Create a startup script that handles Docker socket mounting
RUN cat > /app/start.sh << 'EOF'
#!/bin/sh
set -e

# Check if Docker socket is available
if [ ! -S /var/run/docker.sock ]; then
    echo "Error: Docker socket not found at /var/run/docker.sock"
    echo "Make sure to mount the Docker socket: -v /var/run/docker.sock:/var/run/docker.sock"
    exit 1
fi

# Set Docker host to use the mounted socket
export DOCKER_HOST=unix:///var/run/docker.sock

# Start the service
echo "Starting lambda-at-home-server..."
exec /app/lambda-at-home-server --bind 0.0.0.0 "$@"
EOF

# Make the startup script executable
RUN chmod +x /app/start.sh

# Expose the service ports
EXPOSE 8000 8001 9000

# Set the default command
CMD ["/app/start.sh"]

# Add labels for better container management
LABEL maintainer="Lambda@Home Contributors"
LABEL description="Lambda@Home - Docker-backed AWS Lambda clone"
LABEL version="latest"