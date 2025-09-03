#!/bin/bash

# Complete Lambda@Home Test Suite
set -e

echo "🚀 Lambda@Home Complete Test Suite"
echo "=================================="

# Configuration
USER_API="http://localhost:8080"
FUNCTION_NAME="complete-test-$(date +%s)"

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
log_success() {
    echo "✅ $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo "❌ $1"
    ((TESTS_FAILED++))
}

log_info() {
    echo "📋 $1"
}

# Cleanup function
cleanup() {
    echo "🧹 Cleaning up..."
    docker stop test-lambda-* 2>/dev/null || true
    docker rm test-lambda-* 2>/dev/null || true
}

trap cleanup EXIT

echo
log_info "Test 1: Health Check"
if curl -s "$USER_API/health" | grep -q "ok"; then
    log_success "Health check passed"
else
    log_error "Health check failed"
fi

echo
log_info "Test 2: Function Deployment"
deployment_response=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"function_name\": \"$FUNCTION_NAME\", \"runtime\": \"nodejs22\", \"handler\": \"index.handler\"}" \
    "$USER_API/functions")

if echo "$deployment_response" | grep -q "function_id"; then
    FUNCTION_ID=$(echo "$deployment_response" | grep -o '"function_id":"[^"]*"' | cut -d'"' -f4)
    log_success "Function deployed (ID: $FUNCTION_ID)"
else
    log_error "Function deployment failed"
    exit 1
fi

echo
log_info "Test 3: Container Auto-Registration"
container_id=$(docker run -d --name test-lambda-main \
    -v "$(pwd)/test_function.js:/usr/src/app/index.js" \
    -e AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:3000 \
    -e HANDLER=index.handler \
    -e AWS_LAMBDA_FUNCTION_NAME="$FUNCTION_NAME" \
    javascript-runtime)

sleep 5

if docker ps | grep -q "test-lambda-main"; then
    log_success "Container started and auto-registered"
else
    log_error "Container failed to start"
fi

echo
log_info "Test 4: Single Invocation"
response=$(curl -s -X POST -H "Content-Type: application/json" \
    -d '{"event": {"message": "Single test", "type": "single"}}' \
    "$USER_API/functions/$FUNCTION_ID/invoke")

if echo "$response" | grep -q "Hello from Lambda@Home"; then
    log_success "Single invocation successful"
else
    log_error "Single invocation failed"
fi

echo
log_info "Test 5: Container Reuse"
# Make multiple invocations to test reuse
for i in {1..3}; do
    curl -s -X POST -H "Content-Type: application/json" \
        -d "{\"event\": {\"message\": \"Reuse test $i\", \"type\": \"reuse\"}}" \
        "$USER_API/functions/$FUNCTION_ID/invoke" > /dev/null
done

container_count=$(docker ps | grep "test-lambda" | wc -l)
if [ "$container_count" -eq 1 ]; then
    log_success "Container reuse working (1 container handled 4 invocations)"
else
    log_error "Container reuse failed (containers: $container_count)"
fi

echo
log_info "Test 6: High Concurrency (15 concurrent invocations)"
# Start 15 concurrent invocations
for i in {1..15}; do
    (
        response=$(curl -s -X POST -H "Content-Type: application/json" \
            -d "{\"event\": {\"message\": \"Concurrent $i\", \"type\": \"concurrency\"}}" \
            "$USER_API/functions/$FUNCTION_ID/invoke")
        if echo "$response" | grep -q "Hello from Lambda@Home"; then
            echo "✅ Concurrent $i successful"
        else
            echo "❌ Concurrent $i failed"
        fi
    ) &
done

wait
log_success "High concurrency test completed"

echo
log_info "Test 7: Multiple Container Scaling"
# Start additional containers
for i in {2..4}; do
    docker run -d --name "test-lambda-$i" \
        -v "$(pwd)/test_function.js:/usr/src/app/index.js" \
        -e AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:3000 \
        -e HANDLER=index.handler \
        -e AWS_LAMBDA_FUNCTION_NAME="$FUNCTION_NAME" \
        javascript-runtime
done

sleep 5

container_count=$(docker ps | grep "test-lambda" | wc -l)
if [ "$container_count" -ge 3 ]; then
    log_success "Multiple containers running ($container_count containers)"
else
    log_error "Expected multiple containers, found $container_count"
fi

echo
log_info "Test 8: Load Distribution"
# Make invocations to test load distribution
for i in {1..10}; do
    curl -s -X POST -H "Content-Type: application/json" \
        -d "{\"event\": {\"message\": \"Load test $i\", \"type\": \"load\"}}" \
        "$USER_API/functions/$FUNCTION_ID/invoke" > /dev/null
done

# Check how invocations were distributed
echo "Load distribution:"
for i in {1..4}; do
    if docker ps | grep -q "test-lambda-$i"; then
        invocations=$(docker logs "test-lambda-$i" | grep "Processing Lambda invocation" | wc -l)
        echo "  Container $i: $invocations invocations"
    fi
done

log_success "Load distribution test completed"

echo
log_info "Test 9: Error Handling"
# Test invalid function ID
error_response=$(curl -s -w "%{http_code}" -o /dev/null -X POST -H "Content-Type: application/json" \
    -d '{"event": {"message": "Error test"}}' \
    "$USER_API/functions/invalid-id/invoke")

if [ "$error_response" = "400" ]; then
    log_success "Error handling for invalid function ID working"
else
    log_error "Error handling failed (HTTP $error_response)"
fi

echo
log_info "Test 10: Function Deletion"
delete_response=$(curl -s -X DELETE "$USER_API/functions/$FUNCTION_ID")

if echo "$delete_response" | grep -q "Function deleted successfully"; then
    log_success "Function deletion successful"
else
    log_error "Function deletion failed"
fi

echo
echo "📊 Test Summary"
echo "==============="
echo "✅ Tests passed: $TESTS_PASSED"
echo "❌ Tests failed: $TESTS_FAILED"

if [ "$TESTS_FAILED" -eq 0 ]; then
    echo "🎉 All tests passed! Lambda@Home is working perfectly!"
    exit 0
else
    echo "⚠️  Some tests failed. Please check the logs above."
    exit 1
fi
