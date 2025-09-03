#!/bin/bash

# Lambda@Home Concurrency and Lifecycle Test
set -e

echo "🚀 Testing Lambda@Home Concurrency and Container Lifecycle..."

# Configuration
USER_API="http://localhost:8080"
FUNCTION_NAME="concurrency-test-$(date +%s)"

echo "📋 Step 1: Deploy Function"
deployment_response=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"function_name\": \"$FUNCTION_NAME\", \"runtime\": \"nodejs22\", \"handler\": \"index.handler\"}" \
    "$USER_API/functions")

FUNCTION_ID=$(echo "$deployment_response" | grep -o '"function_id":"[^"]*"' | cut -d'"' -f4)
echo "✅ Function deployed (ID: $FUNCTION_ID)"

echo
echo "🐳 Step 2: Start Multiple Containers"
for i in {1..3}; do
    container_id=$(docker run -d --name "test-lambda-$i" \
        -v "$(pwd)/test_function.js:/usr/src/app/index.js" \
        -e AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:3000 \
        -e HANDLER=index.handler \
        -e AWS_LAMBDA_FUNCTION_NAME="$FUNCTION_NAME" \
        javascript-runtime)
    echo "✅ Container $i started: $container_id"
done

sleep 5

echo
echo "📊 Step 3: Check Container Status"
container_count=$(docker ps | grep "test-lambda" | wc -l)
echo "✅ $container_count containers running"

echo
echo "🔄 Step 4: Test High Concurrency (10 concurrent invocations)"
echo "Starting 10 concurrent invocations..."

# Start 10 concurrent invocations
for i in {1..10}; do
    (
        response=$(curl -s -X POST -H "Content-Type: application/json" \
            -d "{\"event\": {\"message\": \"Concurrent test $i\", \"type\": \"concurrency\", \"id\": $i}}" \
            "$USER_API/functions/$FUNCTION_ID/invoke")
        if echo "$response" | grep -q "Hello from Lambda@Home"; then
            echo "✅ Invocation $i successful"
        else
            echo "❌ Invocation $i failed: $response"
        fi
    ) &
done

# Wait for all invocations to complete
wait

echo
echo "📊 Step 5: Check Container Utilization"
echo "Container logs after concurrency test:"
for i in {1..3}; do
    echo "--- Container $i logs ---"
    docker logs "test-lambda-$i" | grep "Processing Lambda invocation" | wc -l | xargs echo "Invocations processed:"
done

echo
echo "🔄 Step 6: Test Container Reuse"
echo "Making 5 sequential invocations to test container reuse..."

for i in {1..5}; do
    echo "Sequential invocation $i..."
    response=$(curl -s -X POST -H "Content-Type: application/json" \
        -d "{\"event\": {\"message\": \"Sequential test $i\", \"type\": \"sequential\"}}" \
        "$USER_API/functions/$FUNCTION_ID/invoke")
    
    if echo "$response" | grep -q "Hello from Lambda@Home"; then
        echo "✅ Sequential invocation $i successful"
    else
        echo "❌ Sequential invocation $i failed"
    fi
    sleep 1
done

echo
echo "📊 Step 7: Final Container Status"
final_container_count=$(docker ps | grep "test-lambda" | wc -l)
echo "✅ $final_container_count containers still running"

echo
echo "📝 Step 8: Container Logs Summary"
for i in {1..3}; do
    echo "--- Container $i Summary ---"
    total_invocations=$(docker logs "test-lambda-$i" | grep "Processing Lambda invocation" | wc -l)
    echo "Total invocations processed: $total_invocations"
    echo "Last few invocations:"
    docker logs "test-lambda-$i" | grep "Processing Lambda invocation" | tail -3
done

echo
echo "🧹 Step 9: Cleanup"
for i in {1..3}; do
    docker stop "test-lambda-$i" 2>/dev/null || true
    docker rm "test-lambda-$i" 2>/dev/null || true
done

echo "✅ Concurrency and lifecycle test completed successfully!"
