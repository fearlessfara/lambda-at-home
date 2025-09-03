FROM node:18-alpine

# Install runtime interface client
RUN apk add --no-cache curl

# Create runtime directory
RUN mkdir -p /var/runtime /var/task

# Set working directory
WORKDIR /var/task

# Copy bootstrap script
COPY bootstrap.js /var/runtime/bootstrap.js

# Create bootstrap wrapper
RUN echo '#!/bin/sh
set -e
export AWS_LAMBDA_RUNTIME_API=${AWS_LAMBDA_RUNTIME_API:-localhost:9001}
export AWS_LAMBDA_FUNCTION_NAME=${AWS_LAMBDA_FUNCTION_NAME}
export AWS_LAMBDA_FUNCTION_VERSION=${AWS_LAMBDA_FUNCTION_VERSION}
export AWS_LAMBDA_FUNCTION_MEMORY_SIZE=${AWS_LAMBDA_FUNCTION_MEMORY_SIZE}
export AWS_LAMBDA_LOG_GROUP_NAME=${AWS_LAMBDA_LOG_GROUP_NAME}
export AWS_LAMBDA_LOG_STREAM_NAME=${AWS_LAMBDA_LOG_STREAM_NAME}
export LAMBDA_TASK_ROOT=/var/task
export LAMBDA_RUNTIME_DIR=/var/runtime

# Start the runtime
node /var/runtime/bootstrap.js
' > /var/runtime/bootstrap.sh && chmod +x /var/runtime/bootstrap.sh

# Set entrypoint
ENTRYPOINT ["/var/runtime/bootstrap.sh"]

# Set user
USER 1000:1000
