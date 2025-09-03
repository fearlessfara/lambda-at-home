#!/bin/bash

# Simple Lambda@Home Flow Test
set -e

echo "🚀 Testing Lambda@Home Flow..."

# Configuration
USER_API="http://localhost:8080"
FUNCTION_NAME="test-function-$(date +%s)"

echo "📋 Step 1: Deploy Function"
deployment_response=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"function_name\": \"$FUNCTION_NAME\", \"runtime\": \"nodejs22\", \"handler\": \"index.handler\"}" \
    "$USER_API/functions")

echo "Deployment response: $deployment_response"

if echo "$deployment_response" | grep -q "function_id"; then
    FUNCTION_ID=$(echo "$deployment_response" | grep -o '"function_id":"[^"]*"' | cut -d'"' -f4)
    echo "✅ Function deployed successfully (ID: $FUNCTION_ID)"
else
    echo "❌ Function deployment failed"
    exit 1
fi

echo
echo "🐳 Step 2: Start Container"
container_id=$(docker run -d --name test-lambda-simple \
    -v "$(pwd)/test_function.js:/usr/src/app/index.js" \
    -e AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:3000 \
    -e HANDLER=index.handler \
    -e AWS_LAMBDA_FUNCTION_NAME="$FUNCTION_NAME" \
    javascript-runtime)

echo "Container started: $container_id"
sleep 5

echo
echo "📊 Step 3: Check Container Status"
if docker ps | grep -q "test-lambda-simple"; then
    echo "✅ Container is running"
else
    echo "❌ Container failed to start"
    exit 1
fi

echo
echo "📝 Step 4: Check Container Logs"
docker logs test-lambda-simple

echo
echo "🔄 Step 5: Test Invocation"
invocation_response=$(curl -s -X POST -H "Content-Type: application/json" \
    -d '{"event": {"message": "Hello from simple test!", "type": "simple"}}' \
    "$USER_API/functions/$FUNCTION_ID/invoke")

echo "Invocation response: $invocation_response"

if echo "$invocation_response" | grep -q "Hello from Lambda@Home"; then
    echo "✅ Invocation successful"
else
    echo "❌ Invocation failed"
    exit 1
fi

echo
echo "📝 Step 6: Check Container Logs After Invocation"
docker logs test-lambda-simple

echo
echo "🧪 Step 7: Test Multiple Invocations"
for i in {1..3}; do
    echo "Invocation $i..."
    curl -s -X POST -H "Content-Type: application/json" \
        -d "{\"event\": {\"message\": \"Test $i\", \"type\": \"multiple\"}}" \
        "$USER_API/functions/$FUNCTION_ID/invoke" > /dev/null
    sleep 1
done

echo "✅ Multiple invocations completed"

echo
echo "📝 Step 8: Final Container Logs"
docker logs test-lambda-simple

echo
echo "🧹 Step 9: Cleanup"
docker stop test-lambda-simple
docker rm test-lambda-simple

echo "✅ Test completed successfully!"
