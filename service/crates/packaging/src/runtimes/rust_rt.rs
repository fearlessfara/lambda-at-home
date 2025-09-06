use lambda_models::Function;

pub fn dockerfile(function: &Function) -> String {
    format!(
        r#"
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
COPY --from=builder /var/task/target/release/{bin} /var/task/

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
/var/task/{bin}
' > /var/runtime/bootstrap.sh && chmod +x /var/runtime/bootstrap.sh

ENTRYPOINT ["/var/runtime/bootstrap.sh"]
USER 1000:1000
"#,
        bin = function.function_name
    )
}
