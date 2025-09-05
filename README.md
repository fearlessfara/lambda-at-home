# Lambda@Home

A Docker‑backed AWS Lambda clone that runs locally. Lambda@Home provides Lambda‑compatible APIs, a web console, and executes functions in Docker containers with a Lambda‑like lifecycle.

Repository: https://github.com/fearlessfara/lambda-at-home

## Features

- Lambda‑compatible User API and in‑container Runtime API
- Docker‑based isolation for function execution
- Runtimes: Node.js 18/22, Python 3.11, Rust
- Warm pool + reuse: `WarmIdle → Active → WarmIdle`
- Idle management: soft stop and hard removal with watchdog
- Autoscaling: scales to queue depth; restarts stopped instances first
- Concurrency control: global + per‑function reserved concurrency
- API Gateway path proxy with route mappings (prefix + method)
- Web Console: create/update functions, test invoke, manage API routes and Secrets
- Secrets: store once, reference in env as `SECRET_REF:NAME` (masked in UI)
- Metrics: Prometheus endpoint; structured tracing logs
- Security: non‑root, read‑only rootfs, capability drop, tmpfs

## Architecture

Lambda@Home consists of several components:

- **User API** (port 9000): AWS Lambda-compatible REST API
- **Runtime API** (port 9001): In-container runtime interface (RIC)
- **Control Plane**: Function registry, scheduler, warm pool management
- **Invoker**: Docker container lifecycle management
- **Packaging**: ZIP processing and Docker image building
- **Metrics**: Prometheus metrics and structured logging
- **Console**: React app (Vite) for managing functions, API Gateway routes, and Secrets

## Quick Start

[![CI](https://github.com/fearlessfara/lambda-at-home/actions/workflows/ci.yml/badge.svg)](https://github.com/fearlessfara/lambda-at-home/actions/workflows/ci.yml)
[![Release](https://github.com/fearlessfara/lambda-at-home/actions/workflows/release.yml/badge.svg)](https://github.com/fearlessfara/lambda-at-home/actions/workflows/release.yml)

### Prerequisites

- Docker installed and running
- Rust 1.75+ (with rustup)
- Make (optional, for convenience commands)

### Installation

#### For End Users (Recommended)

Download and install the latest binary:

```bash
curl -fsSL https://raw.githubusercontent.com/fearlessfara/lambda-at-home/main/install-lambda-at-home.sh | bash
```

#### For Developers

1) Clone the repository
```bash
git clone https://github.com/fearlessfara/lambda-at-home.git
cd lambda-at-home
```

2) Build the project
```bash
make release
```

### Running the Server

#### For End Users
```bash
lambda-at-home-server
```

#### For Developers
```bash
make run
# or
./target/release/lambda-at-home-server
```

The server will start on:
- User API: http://127.0.0.1:9000/api
- Web Console: http://127.0.0.1:9000 (embedded in binary)
- Runtime API: http://127.0.0.1:9001
- Health: http://127.0.0.1:9000/api/healthz
- Metrics: http://127.0.0.1:9000/api/metrics

### Web Console

The web console is embedded in the binary and available at http://127.0.0.1:9000. No separate setup required!

For development, you can run the console separately:
```bash
cd console
npm install
npm run dev
# open http://localhost:3000
```

Configure the API base URL via `console/.env` (defaults to `/api` in production builds):

```
VITE_API_URL=http://localhost:9000/api
```

### Create and invoke a function (curl)

Package a ZIP with your handler (Node example: `index.js` with `exports.handler`). Then:

Create function
```bash
ZIP_B64=$(base64 < test-function.zip | tr -d '\n')
curl -sS -X POST http://127.0.0.1:9000/2015-03-31/functions \
  -H 'content-type: application/json' \
  -d "{\n    \"function_name\": \"echo\",\n    \"runtime\": \"nodejs18.x\",\n    \"handler\": \"index.handler\",\n    \"code\": { \"zip_file\": \"$ZIP_B64\" }\n  }"
```

Invoke function
```bash
curl -sS -X POST http://127.0.0.1:9000/2015-03-31/functions/echo/invocations \
  -H 'content-type: application/json' \
  -d '{"ping":1}' | jq
```

## Configuration

Configuration is managed via `configs/default.toml`:

```toml
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
```

## Supported Runtimes

### Node.js 18/22
- Runtimes: `nodejs18.x`, `nodejs22.x`
- Handler format: `filename.export`
- Example: `index.handler`

### Python 3.11
- Runtime: `python3.11`
- Handler format: `filename.function`
- Example: `lambda_function.handler`

### Rust
- Runtime: `rust`
- Handler format: `binary_name`
- Example: `lambda`

## API Endpoints

### User API (AWS Lambda Compatible)

- `POST /2015-03-31/functions` - Create function
- `GET /2015-03-31/functions/{name}` - Get function
- `DELETE /2015-03-31/functions/{name}` - Delete function
- `PUT /2015-03-31/functions/{name}/code` - Update function code
- `PUT /2015-03-31/functions/{name}/configuration` - Update function config
- `POST /2015-03-31/functions/{name}/versions` - Publish version
- `GET /2015-03-31/functions` - List functions
- `POST /2015-03-31/functions/{name}/invocations` - Invoke function
- `PUT /2015-03-31/functions/{name}/concurrency` - Set reserved concurrency
- `GET /2015-03-31/functions/{name}/concurrency` - Get reserved concurrency
- `DELETE /2015-03-31/functions/{name}/concurrency` - Clear reserved concurrency
- `GET /api/healthz` - Health check
- `GET /api/metrics` - Prometheus metrics

### API Gateway Path Proxy

Any unmatched path is treated as an API Gateway-style invoke:

- If a configured route mapping matches (longest prefix, optional method) → invokes mapped function
- Else the first URL segment is treated as a function name (if it exists)

Function results are mapped back to HTTP as follows:
- If payload is an object with `statusCode/body/headers` → use those
- If payload is an object with `body` only → return body with status 200
- If payload is a string → return text body
- Otherwise → return JSON payload (status 200)

Admin endpoints for routes:

- `GET /api/admin/api-gateway/routes` – list routes
- `POST /api/admin/api-gateway/routes` – create route `{ path, method?, function_name }`
- `DELETE /api/admin/api-gateway/routes/:id` – delete route

### Runtime API (For Containers)

- `GET /2018-06-01/runtime/invocation/next` - Get next invocation
- `POST /2018-06-01/runtime/invocation/{requestId}/response` - Post response
- `POST /2018-06-01/runtime/invocation/{requestId}/error` - Post error
- `POST /2018-06-01/runtime/init/error` - Post init error

## Security Features

- **Non-root execution**: Containers run as user 1000:1000
- **Read-only rootfs**: Container filesystem is read-only
- **Capability dropping**: All capabilities are dropped
- **No new privileges**: Containers cannot gain new privileges
- **Resource limits**: Memory, CPU, and process limits enforced
- **Tmpfs for /tmp**: Temporary directory with size limits
- **Network isolation**: Containers run in isolated networks

## Development

### Project Structure

```
lambda@home/
├── crates/
│   ├── api/           # User API (AWS Lambda compatible)
│   ├── runtime_api/   # Runtime API (for containers)
│   ├── control/       # Control plane (registry, scheduler)
│   ├── invoker/       # Docker container management
│   ├── packaging/     # ZIP processing and image building
│   ├── models/        # Shared data models
│   ├── metrics/       # Metrics and logging
│   └── cli/           # Command-line tools
├── console/           # Web console (Vite + React)
├── runtimes/          # Runtime Dockerfiles and bootstrap scripts
├── examples/          # Example functions
├── configs/           # Configuration files
├── scripts/           # Curated local test scripts
└── data/              # DB and cache (gitignored)
```

### Running tests

See TESTING.md for details.

### Code Quality

```bash
# Format code
make fmt

# Run clippy
make clippy
```

## Monitoring

### Metrics

Prometheus metrics are available at `/metrics`:

- `lambda_invocations_total` - Total invocations
- `lambda_errors_total` - Total errors
- `lambda_throttles_total` - Total throttles
- `lambda_cold_starts_total` - Total cold starts
- `lambda_duration_ms` - Function execution duration
- `lambda_init_duration_ms` - Function initialization duration

### Logging

Structured JSON logs with fields:
- `ts` - Timestamp
- `level` - Log level
- `message` - Log message
- `function` - Function name
- `version` - Function version
- `req_id` - Request ID
- `container_id` - Container ID
- `duration_ms` - Execution duration
- `billed_ms` - Billed duration
- `mem_peak_mb` - Peak memory usage

## Troubleshooting

### Common Issues

1. **Docker not running**: Ensure Docker is installed and running
2. **Port conflicts**: Check if ports 9000/9001 are available
3. **Permission issues**: Ensure Docker daemon is accessible
4. **Build failures**: Check Rust toolchain and dependencies

### Debug Mode

Run with debug logging:
```bash
RUST_LOG=debug cargo run --bin lambda-at-home-server
```

### Container Logs

Check container logs for function execution issues:
```bash
docker logs <container_id>
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `make test` and `make clippy`
6. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Releases

Tagged builds (vX.Y.Z) trigger the release workflow and publish platform binaries with the web console embedded. Download from GitHub Releases.

Local release build:

```bash
make release
./target/release/lambda-at-home-server
```

## Roadmap
 
### Recently added
- Web Console: functions, testing, API Gateway route management, Secrets
- API Gateway route mappings (prefix + method)
- Per-function reserved concurrency
- Secrets store with `SECRET_REF:NAME` env resolution

### Next up
- Code update flow and versions/aliases UI
- Layers support and more runtimes
- Provisioned concurrency & prewarm controls

- [ ] VPC networking support
- [ ] Provisioned concurrency
- [ ] Layer support
- [ ] More runtime languages
- [ ] WebSocket support
- [ ] Event source mappings
- [ ] Dead letter queues
- [ ] X-Ray tracing integration
