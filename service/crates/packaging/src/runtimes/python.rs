use lambda_models::Function;

pub fn dockerfile(_function: &Function, runtime_api_port: u16) -> String {
    format!(
        r#"
FROM python:3.11-alpine

# Install runtime interface client and WebSocket dependencies
RUN apk add --no-cache curl
RUN pip install --no-cache-dir websockets>=11.0.0

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

# Copy bootstrap scripts
COPY bootstrap.py /var/runtime/bootstrap.py
COPY bootstrap-websocket.py /var/runtime/bootstrap-websocket.py

# Create bootstrap script wrapper
RUN printf '#!/bin/sh\n\
set -e\n\
export AWS_LAMBDA_RUNTIME_API=${{AWS_LAMBDA_RUNTIME_API:-localhost:{runtime_api_port}}}\n\
export AWS_LAMBDA_FUNCTION_NAME=${{AWS_LAMBDA_FUNCTION_NAME}}\n\
export AWS_LAMBDA_FUNCTION_VERSION=${{AWS_LAMBDA_FUNCTION_VERSION}}\n\
export AWS_LAMBDA_FUNCTION_MEMORY_SIZE=${{AWS_LAMBDA_FUNCTION_MEMORY_SIZE}}\n\
export AWS_LAMBDA_LOG_GROUP_NAME=${{AWS_LAMBDA_LOG_GROUP_NAME}}\n\
export AWS_LAMBDA_LOG_STREAM_NAME=${{AWS_LAMBDA_LOG_STREAM_NAME}}\n\
export LAMBDA_TASK_ROOT=/var/task\n\
export LAMBDA_RUNTIME_DIR=/var/runtime\n\
export PYTHONPATH="/var/task:/var/task/python:/opt/python:$PYTHONPATH"\n\
\n\
# Start the runtime\n\
python /var/runtime/bootstrap-websocket.py\n' > /var/runtime/bootstrap.sh && chmod +x /var/runtime/bootstrap.sh

ENTRYPOINT ["/var/runtime/bootstrap.sh"]
USER 1000:1000
"#
    )
}
