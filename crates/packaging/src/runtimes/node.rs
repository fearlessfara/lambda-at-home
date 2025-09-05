use lambda_models::Function;

pub fn dockerfile(function: &Function) -> String {
    let tag = if function.runtime == "nodejs22.x" { "22" } else { "18" };
    format!(r#"
FROM node:{tag}-alpine
ENV NODE_ENV=production

# Install runtime interface client
RUN apk add --no-cache curl

# Create runtime directory
RUN mkdir -p /var/runtime /var/task

# Copy function code
COPY . /var/task/

# Set working directory
WORKDIR /var/task

# Install dependencies if package.json exists (prefer lockfiles)
RUN if [ -f package-lock.json ] || [ -f npm-shrinkwrap.json ]; then \
      npm ci --omit=dev; \
    elif [ -f package.json ]; then \
      npm install --omit=dev; \
    fi && npm cache clean --force

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
"#, tag = tag)
}

