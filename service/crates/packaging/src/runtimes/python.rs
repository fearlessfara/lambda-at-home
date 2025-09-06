use lambda_models::Function;

pub fn dockerfile(_function: &Function) -> String {
    r#"
FROM python:3.11-alpine

# Install runtime interface client
RUN apk add --no-cache curl

# Create runtime directory
RUN mkdir -p /var/runtime /var/task

# Copy function code
COPY . /var/task/

# Set working directory
WORKDIR /var/task

# Install dependencies if not vendored
# Prefer using vendored deps if the package already contains them.
# If not, install into /var/task so imports resolve like AWS Lambda zips.
RUN set -eux; \
    if [ -d "/var/task/python" ]; then \
        echo "Using vendored python deps in /var/task/python"; \
    elif find /var/task -maxdepth 1 -type d -name "*.dist-info" | grep -q . 2>/dev/null; then \
        echo "Using vendored python deps (*.dist-info present)"; \
    elif [ -f /var/task/requirements.txt ]; then \
        pip install --no-cache-dir -r /var/task/requirements.txt -t /var/task; \
    else \
        echo "No requirements.txt and no vendored deps; skipping pip install"; \
    fi

# Copy bootstrap script
COPY bootstrap.py /var/runtime/bootstrap.py

# Create bootstrap script wrapper
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
export PYTHONPATH="/var/task:/var/task/python:/opt/python:$PYTHONPATH"

# Start the runtime
python /var/runtime/bootstrap.py
' > /var/runtime/bootstrap.sh && chmod +x /var/runtime/bootstrap.sh

ENTRYPOINT ["/var/runtime/bootstrap.sh"]
USER 1000:1000
"#
    .to_string()
}
