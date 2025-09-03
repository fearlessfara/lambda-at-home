# Bare Rust Lambda Service

A simple, direct Lambda execution service running as a bare Rust binary without Docker Compose or socat relay.

## 🚀 Quick Start

### 1. Start the Service

```bash
# Build and start the Lambda Runtime API server
./run-bare-service.sh
```

The service will be available at:
- **Lambda Runtime API**: http://localhost:8080
- **Health Check**: http://localhost:8080/health

### 2. Test the Service

```bash
# Run comprehensive tests
./test-bare-service.sh
```

## 🔧 How It Works

### Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Lambda        │    │   Bare Rust      │    │   Docker        │
│   Container     │◄──►│   Service        │◄──►│   Engine        │
│   (Node.js)     │    │   (Port 8080)    │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Key Components

1. **Bare Rust Service**: Runs directly as a binary on the host
2. **Direct Docker Integration**: Creates and manages Lambda containers directly
3. **AWS Lambda Runtime API**: Compatible endpoints for Lambda execution
4. **Container Lifecycle Management**: Handles container states and execution

## 📋 Features

- ✅ **No Docker Compose**: Direct Docker integration
- ✅ **No Socat Relay**: Direct container communication
- ✅ **AWS Lambda Compatible**: Standard Runtime API endpoints
- ✅ **Container Management**: Automatic lifecycle management
- ✅ **Health Monitoring**: Built-in health checks
- ✅ **Concurrent Execution**: Multiple Lambda invocations
- ✅ **Production Ready**: Optimized for performance

## 🌐 API Endpoints

### Health Check
```bash
curl http://localhost:8080/health
```

### Lambda Runtime API
```bash
# Get next invocation
curl http://localhost:8080/runtime/invocation/next

# Submit response
curl -X POST http://localhost:8080/runtime/invocation/{requestId}/response \
  -H "Content-Type: application/json" \
  -d '{"result": "success"}'

# Submit error
curl -X POST http://localhost:8080/runtime/invocation/{requestId}/error \
  -H "Content-Type: application/json" \
  -d '{"errorType": "RuntimeError", "errorMessage": "Something went wrong"}'
```

## 🐳 Lambda Container Setup

### Environment Variables

Lambda containers need these environment variables:

```bash
AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:8080
HANDLER=index.handler
AWS_LAMBDA_FUNCTION_NAME=my-function
AWS_LAMBDA_FUNCTION_MEMORY_SIZE=128
```

### Example Lambda Container

```bash
docker run -d \
  --name my-lambda-container \
  -e AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:8080 \
  -e HANDLER=index.handler \
  -e AWS_LAMBDA_FUNCTION_NAME=my-function \
  -e AWS_LAMBDA_FUNCTION_MEMORY_SIZE=128 \
  my-lambda-image
```

## 🧪 Testing

### Manual Testing

1. **Start the service**:
   ```bash
   ./run-bare-service.sh
   ```

2. **Test health endpoint**:
   ```bash
   curl http://localhost:8080/health
   ```

3. **Create a Lambda container**:
   ```bash
   docker run -d \
     --name test-lambda \
     -e AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:8080 \
     -e HANDLER=index.handler \
     -e AWS_LAMBDA_FUNCTION_NAME=test-function \
     my-lambda-image
   ```

4. **Check container logs**:
   ```bash
   docker logs test-lambda
   ```

### Automated Testing

```bash
# Run comprehensive test suite
./test-bare-service.sh
```

## 🔧 Configuration

### Environment Variables

- `RUST_LOG`: Log level (default: `info`)
- `LAMBDA_RUNTIME_API_PORT`: Server port (default: `8080`)
- `LAMBDA_RUNTIME_API_HOST`: Server host (default: `0.0.0.0`)

### Configuration File

The service uses `config.toml` for configuration:

```toml
[server]
address = "0.0.0.0"
port = 8080

[storage]
path = "/tmp/lambda-functions"

[docker]
network_name = "lambda-network"
container_prefix = "lambda-"

[execution]
max_concurrent_executions = 10
max_memory_mb = 256
max_cpu_shares = 1.0
```

## 🚀 Production Deployment

### Systemd Service

Create `/etc/systemd/system/lambda-runtime-api.service`:

```ini
[Unit]
Description=Lambda Runtime API Server
After=network.target

[Service]
Type=simple
User=lambda
WorkingDirectory=/opt/lambda-runtime-api
ExecStart=/opt/lambda-runtime-api/target/release/lambda-runtime-api-server
Restart=always
RestartSec=5
Environment=RUST_LOG=info
Environment=LAMBDA_RUNTIME_API_PORT=8080

[Install]
WantedBy=multi-user.target
```

### Docker Deployment

```bash
# Build the service
cargo build --release --bin lambda-runtime-api-server

# Run in Docker
docker run -d \
  --name lambda-runtime-api \
  -p 8080:8080 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -e RUST_LOG=info \
  lambda-runtime-api:latest
```

## 🎯 Advantages

### vs Docker Compose Approach
- ✅ **Simpler**: No complex orchestration
- ✅ **Faster**: Direct binary execution
- ✅ **Lighter**: No additional containers
- ✅ **More Control**: Direct Docker API access

### vs Socat Relay Approach
- ✅ **No Network Overhead**: Direct communication
- ✅ **Simpler Setup**: No relay configuration
- ✅ **Better Performance**: No proxy layer
- ✅ **Easier Debugging**: Direct connection

## 🔍 Troubleshooting

### Service Not Starting
```bash
# Check if port is in use
lsof -i :8080

# Check logs
RUST_LOG=debug ./target/release/lambda-runtime-api-server
```

### Container Connection Issues
```bash
# Test container connectivity
docker exec <container> curl http://host.docker.internal:8080/health

# Check Docker networking
docker network ls
```

### Performance Issues
```bash
# Monitor resource usage
htop
docker stats

# Check service logs
journalctl -u lambda-runtime-api -f
```

## 📊 Monitoring

### Health Checks
```bash
# Basic health check
curl http://localhost:8080/health

# Detailed status
curl http://localhost:8080/status
```

### Metrics
- Container count
- Active invocations
- Memory usage
- CPU usage
- Response times

## 🎉 Success!

The bare Rust Lambda service provides a simple, efficient, and production-ready solution for running Lambda functions with direct Docker integration. No complex orchestration needed - just pure Rust performance with Docker container management!
