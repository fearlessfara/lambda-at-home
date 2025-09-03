use anyhow::{Context, Result};
use std::path::Path;
use std::fs;

pub struct RICDockerGenerator;

impl RICDockerGenerator {
    pub fn generate_dockerfile(
        runtime: &str,
        handler: &str,
        working_dir: &str,
    ) -> Result<String> {
        let dockerfile = match runtime.to_lowercase().as_str() {
            "nodejs" | "node" | "javascript" | "js" => {
                Self::generate_nodejs_dockerfile(handler, working_dir)
            }
            "python" | "py" => {
                Self::generate_python_dockerfile(handler, working_dir)
            }
            "go" => {
                Self::generate_go_dockerfile(handler, working_dir)
            }
            _ => {
                return Err(anyhow::anyhow!("Unsupported runtime: {}", runtime));
            }
        };

        Ok(dockerfile)
    }

    pub fn generate_nodejs_dockerfile(handler: &str, working_dir: &str) -> String {
        format!(r#"# Node.js function with Lambda RIC
FROM node:22-slim

# Set working directory
WORKDIR {working_dir}

# Copy user code to a temporary location
COPY . ./user-code/

# Install user dependencies if package.json exists
RUN if [ -f user-code/package.json ]; then \
        cd user-code && \
        npm install --only=production; \
    fi

# Copy user code to the working directory
RUN cp -r user-code/* . 2>/dev/null || true && \
    rm -rf user-code

# Copy Lambda RIC runtime
COPY runtimes/nodejs-ric/package*.json ./
COPY runtimes/nodejs-ric/src/ ./src/

# Install Lambda RIC dependencies
RUN npm install --only=production

# Set function-specific environment variables
ENV HANDLER={handler}
ENV AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:3000
ENV CONTAINER_ID=container-$(date +%s)

# Expose port 8080 for health checks
EXPOSE 8080

# Start the Lambda RIC server
CMD ["node", "src/index.js"]
"#)
    }

    pub fn generate_python_dockerfile(handler: &str, working_dir: &str) -> String {
        format!(r#"# Python function with RIC
FROM python-runtime

# Set working directory
WORKDIR {working_dir}

# Copy requirements first for better Docker layer caching
COPY requirements.txt ./

# Install dependencies if requirements.txt exists
RUN if [ -f requirements.txt ]; then pip install --no-cache-dir -r requirements.txt; fi

# Copy function code
COPY . .

# Set function-specific environment variables
ENV HANDLER={handler}

# RIC server is already installed and configured in the base image
# Port 8080 is already exposed

# Start RIC server (already configured in base image)
CMD ["python", "src/runtime.py"]
"#)
    }

    pub fn generate_go_dockerfile(handler: &str, working_dir: &str) -> String {
        format!(r#"# Go function with RIC
FROM golang:1.21-slim

# Set working directory for function code
WORKDIR {working_dir}

# Copy go mod files first for better Docker layer caching
COPY go.mod go.sum ./

# Download dependencies if go.mod exists
RUN if [ -f go.mod ]; then go mod download; fi

# Copy function code
COPY . .

# Set function-specific environment variables
ENV HANDLER={handler}

# Build and run function
RUN go build -o main .
CMD ["./main"]
"#)
    }

    pub fn write_dockerfile_to_path(
        dockerfile_path: &Path,
        runtime: &str,
        handler: &str,
        working_dir: &str,
    ) -> Result<()> {
        let dockerfile_content = Self::generate_dockerfile(runtime, handler, working_dir)?;
        fs::write(dockerfile_path, dockerfile_content)
            .with_context(|| format!("Failed to write Dockerfile to {:?}", dockerfile_path))?;
        Ok(())
    }
}