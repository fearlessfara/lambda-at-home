use lambda_models::Function;

pub fn dockerfile(function: &Function, runtime_api_port: u16) -> String {
    let tag = match function.runtime.as_str() {
        "nodejs24.x" => "24",
        "nodejs22.x" => "22",
        _ => "18",
    };
    format!(
        r#"
FROM node:{tag}-alpine
ENV NODE_ENV=production

# Install runtime interface client and WebSocket dependencies
RUN apk add --no-cache curl
RUN mkdir -p /var/runtime/node_modules && npm install ws --prefix /var/runtime

# Create runtime directory
RUN mkdir -p /var/runtime /var/task

# Copy function code
COPY . /var/task/

# Set working directory
WORKDIR /var/task

# Install dependencies only if not vendored
# - If node_modules already exists in the package, use it as-is
# - Else prefer lockfiles for reproducible installs
RUN if [ -d node_modules ]; then \
      echo "Using vendored node_modules"; \
    elif [ -f package-lock.json ] || [ -f npm-shrinkwrap.json ]; then \
      npm ci --omit=dev; \
    elif [ -f package.json ]; then \
      npm install --omit=dev; \
    else \
      echo "No package.json found; skipping npm install"; \
    fi && npm cache clean --force || true

# Copy bootstrap scripts
COPY bootstrap.js /var/runtime/bootstrap.js
COPY bootstrap-websocket.js /var/runtime/bootstrap-websocket.js

# Create bootstrap wrapper
RUN printf '#!/bin/sh\n\
set -e\n\
export AWS_LAMBDA_RUNTIME_API=${{AWS_LAMBDA_RUNTIME_API:-host.docker.internal:{runtime_api_port}}}\n\
export AWS_LAMBDA_FUNCTION_NAME=${{AWS_LAMBDA_FUNCTION_NAME}}\n\
export AWS_LAMBDA_FUNCTION_VERSION=${{AWS_LAMBDA_FUNCTION_VERSION}}\n\
export AWS_LAMBDA_FUNCTION_MEMORY_SIZE=${{AWS_LAMBDA_FUNCTION_MEMORY_SIZE}}\n\
export AWS_LAMBDA_LOG_GROUP_NAME=${{AWS_LAMBDA_LOG_GROUP_NAME}}\n\
export AWS_LAMBDA_LOG_STREAM_NAME=${{AWS_LAMBDA_LOG_STREAM_NAME}}\n\
export LAMBDA_TASK_ROOT=/var/task\n\
export LAMBDA_RUNTIME_DIR=/var/runtime\n\
export NODE_PATH="/var/task/node_modules:/opt/nodejs/node_modules:/opt/node_modules:$NODE_PATH"\n\
\n\
# Start the runtime\n\
node /var/runtime/bootstrap-websocket.js\n' > /var/runtime/bootstrap.sh && chmod +x /var/runtime/bootstrap.sh

# Set entrypoint
ENTRYPOINT ["/var/runtime/bootstrap.sh"]

# Set user
USER 1000:1000
"#
    )
}
