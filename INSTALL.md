# Lambda@Home Installation Guide

This guide will help you set up Lambda@Home on your system.

## Prerequisites

Before installing Lambda@Home, make sure you have the following installed:

- **Docker** - Required for running lambda function containers
  - Install from: https://docs.docker.com/get-docker/
  - Make sure Docker daemon is running

- **Rust** - Required for building Lambda@Home
  - Install from: https://rustup.rs/
  - Version 1.70+ recommended

## Quick Setup

### Option 1: Using the Setup Script (Recommended)

1. Clone or download the Lambda@Home repository
2. Run the setup script:
   ```bash
   ./setup.sh
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```
4. Run Lambda@Home:
   ```bash
   cargo run
   ```

### Option 2: Using the Full Install Script

1. Clone or download the Lambda@Home repository
2. Run the install script:
   ```bash
   ./install.sh
   ```
3. Build the project:
   ```bash
   cargo build --release
   ```
4. Run Lambda@Home:
   ```bash
   cargo run
   ```

### Option 3: Manual Setup

If you prefer to set up manually:

1. Create the required directories:
   ```bash
   mkdir -p data/cache data/zips config functions
   ```

2. Create a configuration file at `config/config.toml`:
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

3. Build and run:
   ```bash
   cargo build --release
   cargo run
   ```

## Verification

After installation, Lambda@Home should be running and accessible at:

- **User API**: http://127.0.0.1:9000
- **Runtime API**: http://127.0.0.1:9001

You can test the installation by making a request to the health endpoint:

```bash
curl http://127.0.0.1:9000/health
```

## Directory Structure

After installation, your Lambda@Home directory should look like this:

```
lambda@home/
├── data/                    # Runtime data directory
│   ├── cache/              # Docker image cache
│   ├── zips/               # Function code storage
│   ├── lhome.db            # SQLite database (created on first run)
│   └── .gitignore          # Git ignore file for data
├── config/                 # Configuration directory
│   └── config.toml         # Main configuration file
├── functions/              # User function storage
├── crates/                 # Source code
├── runtimes/               # Runtime templates
└── target/                 # Build output
```

## Configuration

The main configuration file is located at `config/config.toml`. Key settings:

- **Server ports**: Change `port_user_api` and `port_runtime_api` to use different ports
- **Memory limits**: Adjust `memory_mb` for default function memory allocation
- **Timeouts**: Modify `timeout_ms` for default function timeout
- **Idle settings**: Configure `soft_ms` and `hard_ms` for container lifecycle management
- **Concurrency**: Set `max_global_concurrency` for maximum concurrent executions

## Troubleshooting

### Database Error: "unable to open database file"

This error occurs when the data directory doesn't exist or has incorrect permissions. Run the setup script to fix:

```bash
./setup.sh
```

### Docker Not Found

Make sure Docker is installed and running:

```bash
# Check if Docker is installed
docker --version

# Check if Docker daemon is running
docker info
```

### Permission Denied

Make sure the data directory has proper permissions:

```bash
chmod 755 data
chmod 755 data/cache
chmod 755 data/zips
```

### Port Already in Use

If ports 9000 or 9001 are already in use, modify the configuration:

```toml
[server]
port_user_api = 9002
port_runtime_api = 9003
```

## System Service (Optional)

For production deployments, you can set up Lambda@Home as a system service:

1. Run the install script as root (it will create a systemd service file)
2. Enable the service:
   ```bash
   sudo systemctl enable lambda-at-home
   ```
3. Start the service:
   ```bash
   sudo systemctl start lambda-at-home
   ```

## Uninstallation

To remove Lambda@Home:

1. Stop the service (if running as a service):
   ```bash
   sudo systemctl stop lambda-at-home
   sudo systemctl disable lambda-at-home
   ```

2. Remove the service file:
   ```bash
   sudo rm /etc/systemd/system/lambda-at-home.service
   ```

3. Remove the project directory:
   ```bash
   rm -rf /path/to/lambda@home
   ```

## Support

For issues and questions:

1. Check the troubleshooting section above
2. Review the logs for error messages
3. Open an issue on the project repository
4. Check the main README.md for additional documentation

## Next Steps

After successful installation:

1. Read the main README.md for usage instructions
2. Try creating your first lambda function
3. Explore the examples in the `examples/` directory
4. Check out the web console at http://127.0.0.1:9000 (if available)
