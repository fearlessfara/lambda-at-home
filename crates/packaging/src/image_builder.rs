
use std::process::Stdio;
use tokio::process::Command;
use lambda_models::{Function, LambdaError};
use crate::zip_handler::ZipInfo;
use tracing::{info, error, instrument};

pub struct ImageBuilder {
    docker_host: String,
}

impl ImageBuilder {
    pub fn new(docker_host: String) -> Self {
        Self { docker_host }
    }

    #[instrument(skip(self, function, zip_info))]
    pub async fn build_image(&self, function: &Function, zip_info: &ZipInfo, image_ref: &str) -> Result<(), LambdaError> {
        
        // Create temporary directory for build context
        let temp_dir = tempfile::tempdir()
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        let build_context = temp_dir.path();
        
        // Extract ZIP to build context
        let zip_handler = crate::zip_handler::ZipHandler::new(50 * 1024 * 1024); // 50MB limit
        zip_handler.extract_to_directory(&zip_info.zip_data, build_context).await?;
        
        // Copy bootstrap script to build context
        let bootstrap_source = std::env::current_dir()
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?
            .join("runtimes")
            .join("nodejs18")
            .join("bootstrap.js");
        
        if bootstrap_source.exists() {
            let bootstrap_dest = build_context.join("bootstrap.js");
            std::fs::copy(&bootstrap_source, &bootstrap_dest)
                .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        }
        
        // Create Dockerfile based on runtime
        let dockerfile_content = self.create_dockerfile(function)?;
        let dockerfile_path = build_context.join("Dockerfile");
        std::fs::write(&dockerfile_path, dockerfile_content)
            .map_err(|e| LambdaError::InternalError { reason: e.to_string() })?;
        
        // Build Docker image
        info!("Building Docker image: {}", image_ref);
        info!("Build context: {:?}", build_context);
        info!("Dockerfile path: {:?}", dockerfile_path);
        
        let build_result = Command::new("docker")
            .arg("build")
            .arg("-t")
            .arg(image_ref)
            .arg("-f")
            .arg(&dockerfile_path)
            .arg(build_context)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| LambdaError::DockerError { message: e.to_string() })?;
        
        if !build_result.status.success() {
            let stdout = String::from_utf8_lossy(&build_result.stdout);
            let stderr = String::from_utf8_lossy(&build_result.stderr);
            error!("Docker build failed - stdout: {}", stdout);
            error!("Docker build failed - stderr: {}", stderr);
            return Err(LambdaError::DockerError { 
                message: format!("Docker build failed: {}", stderr) 
            });
        }
        
        info!("Built Docker image: {}", image_ref);
        Ok(())
    }

    fn create_dockerfile(&self, function: &Function) -> Result<String, LambdaError> {
        match function.runtime.as_str() {
            "nodejs18.x" => Ok(self.create_nodejs_dockerfile(function)),
            "python3.11" => Ok(self.create_python_dockerfile(function)),
            "rust" => Ok(self.create_rust_dockerfile(function)),
            _ => Err(LambdaError::InvalidRuntime { runtime: function.runtime.clone() }),
        }
    }

    fn create_nodejs_dockerfile(&self, function: &Function) -> String {
        format!(r#"
FROM node:18-alpine

# Install runtime interface client
RUN apk add --no-cache curl

# Create runtime directory
RUN mkdir -p /var/runtime /var/task

# Copy function code
COPY . /var/task/

# Set working directory
WORKDIR /var/task

# Install dependencies if package.json exists
RUN if [ -f package.json ]; then npm install --production; fi

# Copy bootstrap script
COPY bootstrap.js /var/runtime/bootstrap.js

# Create bootstrap wrapper
RUN printf '#!/bin/sh\n\
set -e\n\
export AWS_LAMBDA_RUNTIME_API=${{AWS_LAMBDA_RUNTIME_API:-host.docker.internal:9001}}\n\
export AWS_LAMBDA_FUNCTION_NAME=${{AWS_LAMBDA_FUNCTION_NAME}}\n\
export AWS_LAMBDA_FUNCTION_VERSION=${{AWS_LAMBDA_FUNCTION_VERSION}}\n\
export AWS_LAMBDA_FUNCTION_MEMORY_SIZE=${{AWS_LAMBDA_FUNCTION_MEMORY_SIZE}}\n\
export AWS_LAMBDA_LOG_GROUP_NAME=${{AWS_LAMBDA_LOG_GROUP_NAME}}\n\
export AWS_LAMBDA_LOG_STREAM_NAME=${{AWS_LAMBDA_LOG_STREAM_NAME}}\n\
export LAMBDA_TASK_ROOT=/var/task\n\
export LAMBDA_RUNTIME_DIR=/var/runtime\n\
\n\
# Start the runtime\n\
node /var/runtime/bootstrap.js\n' > /var/runtime/bootstrap.sh && chmod +x /var/runtime/bootstrap.sh

# Set entrypoint
ENTRYPOINT ["/var/runtime/bootstrap.sh"]

# Set user
USER 1000:1000
"#)
    }

    fn create_python_dockerfile(&self, function: &Function) -> String {
        format!(r#"
FROM python:3.11-alpine

# Install runtime interface client
RUN apk add --no-cache curl

# Create runtime directory
RUN mkdir -p /var/runtime /var/task

# Copy function code
COPY . /var/task/

# Set working directory
WORKDIR /var/task

# Install dependencies if requirements.txt exists
RUN if [ -f requirements.txt ]; then pip install --no-cache-dir -r requirements.txt; fi

# Create bootstrap script
RUN echo '#!/bin/sh
set -e
export AWS_LAMBDA_RUNTIME_API=${{AWS_LAMBDA_RUNTIME_API:-localhost:9001}}
export AWS_LAMBDA_FUNCTION_NAME=${{AWS_LAMBDA_FUNCTION_NAME}}
export AWS_LAMBDA_FUNCTION_VERSION=${{AWS_LAMBDA_FUNCTION_VERSION}}
export AWS_LAMBDA_FUNCTION_MEMORY_SIZE=${{AWS_LAMBDA_FUNCTION_MEMORY_SIZE}}
export AWS_LAMBDA_LOG_GROUP_NAME=${{AWS_LAMBDA_LOG_GROUP_NAME}}
export AWS_LAMBDA_LOG_STREAM_NAME=${{AWS_LAMBDA_LOG_STREAM_NAME}}
export LAMBDA_TASK_ROOT=/var/task
export LAMBDA_RUNTIME_DIR=/var/runtime

# Start the runtime
python /var/runtime/bootstrap.py
' > /var/runtime/bootstrap.sh && chmod +x /var/runtime/bootstrap.sh

# Set entrypoint
ENTRYPOINT ["/var/runtime/bootstrap.sh"]

# Set user
USER 1000:1000
"#)
    }

    fn create_rust_dockerfile(&self, function: &Function) -> String {
        format!(r#"
FROM rust:1.75-alpine as builder

# Install build dependencies
RUN apk add --no-cache musl-dev

# Create runtime directory
RUN mkdir -p /var/runtime /var/task

# Copy function code
COPY . /var/task/

# Set working directory
WORKDIR /var/task

# Build the function
RUN cargo build --release

# Runtime stage
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache libgcc

# Create runtime directory
RUN mkdir -p /var/runtime /var/task

# Copy built binary
COPY --from=builder /var/task/target/release/{} /var/task/

# Create bootstrap script
RUN echo '#!/bin/sh
set -e
export AWS_LAMBDA_RUNTIME_API=${{AWS_LAMBDA_RUNTIME_API:-localhost:9001}}
export AWS_LAMBDA_FUNCTION_NAME=${{AWS_LAMBDA_FUNCTION_NAME}}
export AWS_LAMBDA_FUNCTION_VERSION=${{AWS_LAMBDA_FUNCTION_VERSION}}
export AWS_LAMBDA_FUNCTION_MEMORY_SIZE=${{AWS_LAMBDA_FUNCTION_MEMORY_SIZE}}
export AWS_LAMBDA_LOG_GROUP_NAME=${{AWS_LAMBDA_LOG_GROUP_NAME}}
export AWS_LAMBDA_LOG_STREAM_NAME=${{AWS_LAMBDA_LOG_STREAM_NAME}}
export LAMBDA_TASK_ROOT=/var/task
export LAMBDA_RUNTIME_DIR=/var/runtime

# Start the runtime
/var/task/{}
' > /var/runtime/bootstrap.sh && chmod +x /var/runtime/bootstrap.sh

# Set entrypoint
ENTRYPOINT ["/var/runtime/bootstrap.sh"]

# Set user
USER 1000:1000
"#, function.function_name, function.function_name)
    }
}
