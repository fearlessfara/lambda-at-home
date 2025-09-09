# WebSocket Runtime Test Function (Python)

This example demonstrates how to use the WebSocket-enabled Lambda runtime with Python.

## Features

- Uses WebSocket connection for faster communication with the runtime API
- Automatic fallback to HTTP if WebSocket is not available
- Includes the `websockets` dependency for WebSocket support

## Usage

1. Package the function:
   ```bash
   zip -r websocket-test-function-python.zip .
   ```

2. Create the function with WebSocket support enabled:
   ```bash
   ZIP_B64=$(base64 < websocket-test-function-python.zip | tr -d '\n')
   curl -sS -X POST http://127.0.0.1:9000/2015-03-31/functions \
     -H 'content-type: application/json' \
     -d "{
       \"function_name\": \"websocket-test-python\",
       \"runtime\": \"python311\",
       \"handler\": \"lambda_function.handler\",
       \"code\": { \"zip_file\": \"$ZIP_B64\" }
     }"
   ```

3. Invoke the function:
   ```bash
   curl -sS -X POST http://127.0.0.1:9000/2015-03-31/functions/websocket-test-python/invocations \
     -H 'content-type: application/json' \
     -d '{"test": "websocket", "language": "python"}' | jq
   ```

## Environment Variables

- `LAMBDA_USE_WEBSOCKET=true` - Enable WebSocket runtime (default: true)
- `LAMBDA_USE_WEBSOCKET=false` - Force HTTP runtime

## Performance Benefits

The WebSocket runtime provides:
- Lower latency for function invocations
- Reduced connection overhead
- Better resource utilization
- Persistent connections during container warm periods
