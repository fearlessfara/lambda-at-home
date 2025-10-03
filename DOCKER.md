# Docker Setup for Lambda@Home

This directory contains Docker configuration files to run Lambda@Home in a containerized environment while using the host's Docker daemon.

## Files

- `Dockerfile` - Production Dockerfile that downloads the latest release binary
- `Dockerfile.dev` - Development Dockerfile that builds from source
- `docker-compose.yml` - Main Docker Compose configuration
- `docker-compose.override.yml` - Override file for custom configurations

## Quick Start

### Using Docker Compose (Recommended)

1. **Clone the repository:**
   ```bash
   git clone https://github.com/fearlessfara/lambda-at-home.git
   cd lambda-at-home
   ```

2. **Start the service:**
   ```bash
   docker-compose up -d
   ```

3. **Check the service status:**
   ```bash
   docker-compose ps
   docker-compose logs -f
   ```

4. **Access the service:**
   - User API: http://localhost:8000
   - Runtime API: http://localhost:8001
   - Health check: http://localhost:8000/health

### Using Docker directly

1. **Build the image:**
   ```bash
   docker build -t lambda-at-home .
   ```

2. **Run the container:**
   ```bash
   docker run -d \
     --name lambda-at-home \
     -p 8000:8000 \
     -p 8001:8001 \
     -v /var/run/docker.sock:/var/run/docker.sock:ro \
     -v lambda-at-home-data:/app/data \
     lambda-at-home
   ```

## Architecture Support

The Dockerfile automatically detects the host architecture and downloads the appropriate binary:
- **x86_64**: Downloads `linux-x86_64` binary
- **ARM64**: Downloads `linux-arm64` binary

## Configuration

### Environment Variables

- `RUST_LOG` - Log level (default: `info`)
- `LAMBDA_AT_HOME_DATA_DIR` - Data directory path (default: `/app/data`)
- `DOCKER_HOST` - Docker daemon socket (default: `unix:///var/run/docker.sock`)

### Volumes

- `/var/run/docker.sock` - Docker socket (required, read-only)
- `lambda-at-home-data` - Persistent data storage
- Optional: Custom config file mounting

### Ports

- `8000` - User API (AWS Lambda-compatible API)
- `8001` - Runtime API (for function containers)

## Development

### Using Development Dockerfile

For development with live code changes:

1. **Build the development image:**
   ```bash
   docker build -f Dockerfile.dev -t lambda-at-home:dev .
   ```

2. **Run with source code mounted:**
   ```bash
   docker run -d \
     --name lambda-at-home-dev \
     -p 8000:8000 \
     -p 8001:8001 \
     -v /var/run/docker.sock:/var/run/docker.sock:ro \
     -v lambda-at-home-data:/app/data \
     -v $(pwd)/service:/app/service \
     lambda-at-home:dev
   ```

### Custom Configuration

1. **Copy the override file:**
   ```bash
   cp docker-compose.override.yml.example docker-compose.override.yml
   ```

2. **Modify the configuration as needed**

3. **Start with custom config:**
   ```bash
   docker-compose up -d
   ```

## Troubleshooting

### Common Issues

1. **Docker socket permission denied:**
   ```bash
   sudo chmod 666 /var/run/docker.sock
   # Or add your user to the docker group
   sudo usermod -aG docker $USER
   ```

2. **Port already in use:**
   ```bash
   # Check what's using the port
   lsof -i :8000
   # Kill the process or change the port in docker-compose.yml
   ```

3. **Container fails to start:**
   ```bash
   # Check logs
   docker-compose logs lambda-at-home
   # Check if Docker daemon is running
   docker info
   ```

### Health Checks

The service includes health checks that verify the API is responding:

```bash
# Check container health
docker-compose ps

# Manual health check
curl http://localhost:8000/health
```

### Logs

```bash
# View logs
docker-compose logs -f lambda-at-home

# View logs with timestamps
docker-compose logs -f -t lambda-at-home
```

## Security Considerations

- The container runs with access to the host's Docker daemon
- Only mount the Docker socket as read-only when possible
- Consider using Docker-in-Docker (DinD) for production environments
- Regularly update the base image and dependencies

## Production Deployment

For production deployments, consider:

1. **Using a specific release tag instead of latest**
2. **Implementing proper secrets management**
3. **Setting up monitoring and alerting**
4. **Using a reverse proxy (nginx, traefik)**
5. **Implementing backup strategies for the data volume**

## Examples

### Basic Usage

```bash
# Start the service
docker-compose up -d

# Create a function
curl -X POST http://localhost:8000/2015-03-31/functions \
  -H "Content-Type: application/json" \
  -d '{
    "FunctionName": "hello-world",
    "Runtime": "nodejs24",
    "Handler": "index.handler",
    "Code": {
      "ZipFile": "base64-encoded-zip-content"
    }
  }'

# Invoke the function
curl -X POST http://localhost:8000/2015-03-31/functions/hello-world/invocations \
  -H "Content-Type: application/json" \
  -d '{"key": "value"}'
```

### Stopping the Service

```bash
# Stop the service
docker-compose down

# Stop and remove volumes (WARNING: This will delete all data)
docker-compose down -v
```
