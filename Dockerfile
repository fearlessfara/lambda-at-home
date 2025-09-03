# Use a multi-stage build
FROM rust:slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the Rust project
COPY . .

# Build the Lambda Runtime API server
RUN cargo build --release --bin lambda-runtime-api-server

# Final stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN useradd -m -s /bin/bash lambda-user

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/lambda-runtime-api-server /app/

# Create directories for the Lambda service
RUN mkdir -p /app/data /app/logs && \
    chown -R lambda-user:lambda-user /app

# Switch to non-root user
USER lambda-user

# Expose the Lambda Runtime API port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info

# Run the Lambda Runtime API server
CMD ["./lambda-runtime-api-server"]
