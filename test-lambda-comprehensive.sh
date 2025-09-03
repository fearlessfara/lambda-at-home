#!/bin/bash

# Comprehensive Lambda@Home Test Suite
# Tests: Function deployment, invocation, concurrency, container lifecycle

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
USER_API="http://localhost:8080"
RIC_API="http://localhost:3000"
TEST_FUNCTION_NAME="test-function-$(date +%s)"
TEST_RUNTIME="nodejs22"
TEST_HANDLER="index.handler"

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
    ((TESTS_PASSED++))
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    ((TESTS_FAILED++))
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test containers..."
    docker stop test-lambda-* 2>/dev/null || true
    docker rm test-lambda-* 2>/dev/null || true
}

# Set trap for cleanup
trap cleanup EXIT

# Test 1: Health Check
test_health_check() {
    log_info "Testing health check..."
    
    response=$(curl -s -w "%{http_code}" -o /dev/null "$USER_API/health")
    if [ "$response" = "200" ]; then
        log_success "Health check passed"
    else
        log_error "Health check failed (HTTP $response)"
        return 1
    fi
}

# Test 2: Function Deployment
test_function_deployment() {
    log_info "Testing function deployment..."
    
    deployment_response=$(curl -s -X POST -H "Content-Type: application/json" \
        -d "{\"function_name\": \"$TEST_FUNCTION_NAME\", \"runtime\": \"$TEST_RUNTIME\", \"handler\": \"$TEST_HANDLER\"}" \
        "$USER_API/functions")
    
    if echo "$deployment_response" | grep -q "function_id"; then
        FUNCTION_ID=$(echo "$deployment_response" | grep -o '"function_id":"[^"]*"' | cut -d'"' -f4)
        log_success "Function deployed successfully (ID: $FUNCTION_ID)"
        echo "$FUNCTION_ID" > /tmp/test_function_id
    else
        log_error "Function deployment failed: $deployment_response"
        return 1
    fi
}

# Test 3: Container Startup and Auto-Registration
test_container_startup() {
    log_info "Testing container startup and auto-registration..."
    
    # Start a container
    container_id=$(docker run -d --name test-lambda-1 \
        -v "$(pwd)/test_function.js:/usr/src/app/index.js" \
        -e AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:3000 \
        -e HANDLER=index.handler \
        -e AWS_LAMBDA_FUNCTION_NAME="$TEST_FUNCTION_NAME" \
        javascript-runtime)
    
    # Wait for container to start and register
    sleep 5
    
    # Check if container is running
    if docker ps | grep -q "test-lambda-1"; then
        log_success "Container started successfully"
    else
        log_error "Container failed to start"
        return 1
    fi
    
    # Check container logs for successful registration
    if docker logs test-lambda-1 2>&1 | grep -q "Lambda RIC initialization completed"; then
        log_success "Container auto-registered successfully"
    else
        log_error "Container failed to auto-register"
        return 1
    fi
}

# Test 4: Single Function Invocation
test_single_invocation() {
    log_info "Testing single function invocation..."
    
    FUNCTION_ID=$(cat /tmp/test_function_id)
    
    invocation_response=$(curl -s -X POST -H "Content-Type: application/json" \
        -d '{"event": {"message": "Hello from single invocation test!", "type": "single"}}' \
        "$USER_API/functions/$FUNCTION_ID/invoke")
    
    if echo "$invocation_response" | grep -q "Hello from Lambda@Home"; then
        log_success "Single invocation successful"
    else
        log_error "Single invocation failed: $invocation_response"
        return 1
    fi
}

# Test 5: Concurrent Invocations
test_concurrent_invocations() {
    log_info "Testing concurrent invocations..."
    
    FUNCTION_ID=$(cat /tmp/test_function_id)
    
    # Start multiple concurrent invocations
    for i in {1..5}; do
        (
            curl -s -X POST -H "Content-Type: application/json" \
                -d "{\"event\": {\"message\": \"Concurrent test $i\", \"type\": \"concurrent\", \"id\": $i}}" \
                "$USER_API/functions/$FUNCTION_ID/invoke" > "/tmp/concurrent_response_$i.json" &
        ) &
    done
    
    # Wait for all invocations to complete
    wait
    
    # Check results
    success_count=0
    for i in {1..5}; do
        if [ -f "/tmp/concurrent_response_$i.json" ] && grep -q "Hello from Lambda@Home" "/tmp/concurrent_response_$i.json"; then
            ((success_count++))
        fi
    done
    
    if [ "$success_count" -eq 5 ]; then
        log_success "All 5 concurrent invocations successful"
    else
        log_error "Only $success_count/5 concurrent invocations successful"
        return 1
    fi
}

# Test 6: Container Reuse
test_container_reuse() {
    log_info "Testing container reuse..."
    
    FUNCTION_ID=$(cat /tmp/test_function_id)
    
    # Get initial container count
    initial_containers=$(docker ps | grep "test-lambda" | wc -l)
    
    # Make multiple invocations
    for i in {1..3}; do
        curl -s -X POST -H "Content-Type: application/json" \
            -d "{\"event\": {\"message\": \"Reuse test $i\", \"type\": \"reuse\"}}" \
            "$USER_API/functions/$FUNCTION_ID/invoke" > /dev/null
        sleep 1
    done
    
    # Check if container count remained the same (reuse)
    final_containers=$(docker ps | grep "test-lambda" | wc -l)
    
    if [ "$final_containers" -eq "$initial_containers" ]; then
        log_success "Container reuse working (containers: $initial_containers)"
    else
        log_warning "Container count changed from $initial_containers to $final_containers (may be expected for scaling)"
    fi
}

# Test 7: Multiple Container Scaling
test_container_scaling() {
    log_info "Testing container scaling..."
    
    FUNCTION_ID=$(cat /tmp/test_function_id)
    
    # Start additional containers
    for i in {2..3}; do
        docker run -d --name "test-lambda-$i" \
            -v "$(pwd)/test_function.js:/usr/src/app/index.js" \
            -e AWS_LAMBDA_RUNTIME_API=http://host.docker.internal:3000 \
            -e HANDLER=index.handler \
            -e AWS_LAMBDA_FUNCTION_NAME="$TEST_FUNCTION_NAME" \
            javascript-runtime
    done
    
    sleep 5
    
    # Check if multiple containers are running
    container_count=$(docker ps | grep "test-lambda" | wc -l)
    
    if [ "$container_count" -ge 2 ]; then
        log_success "Multiple containers running ($container_count containers)"
    else
        log_error "Expected multiple containers, found $container_count"
        return 1
    fi
}

# Test 8: High Concurrency Test
test_high_concurrency() {
    log_info "Testing high concurrency (10 concurrent invocations)..."
    
    FUNCTION_ID=$(cat /tmp/test_function_id)
    
    # Start 10 concurrent invocations
    for i in {1..10}; do
        (
            curl -s -X POST -H "Content-Type: application/json" \
                -d "{\"event\": {\"message\": \"High concurrency test $i\", \"type\": \"high_concurrency\", \"id\": $i}}" \
                "$USER_API/functions/$FUNCTION_ID/invoke" > "/tmp/high_concurrent_response_$i.json" &
        ) &
    done
    
    # Wait for all invocations to complete
    wait
    
    # Check results
    success_count=0
    for i in {1..10}; do
        if [ -f "/tmp/high_concurrent_response_$i.json" ] && grep -q "Hello from Lambda@Home" "/tmp/high_concurrent_response_$i.json"; then
            ((success_count++))
        fi
    done
    
    if [ "$success_count" -eq 10 ]; then
        log_success "All 10 high concurrency invocations successful"
    else
        log_error "Only $success_count/10 high concurrency invocations successful"
        return 1
    fi
}

# Test 9: Error Handling
test_error_handling() {
    log_info "Testing error handling..."
    
    # Test invalid function ID
    error_response=$(curl -s -w "%{http_code}" -o /dev/null -X POST -H "Content-Type: application/json" \
        -d '{"event": {"message": "Error test"}}' \
        "$USER_API/functions/invalid-function-id/invoke")
    
    if [ "$error_response" = "400" ]; then
        log_success "Error handling for invalid function ID working"
    else
        log_error "Error handling failed (HTTP $error_response)"
        return 1
    fi
}

# Test 10: Function Deletion
test_function_deletion() {
    log_info "Testing function deletion..."
    
    FUNCTION_ID=$(cat /tmp/test_function_id)
    
    delete_response=$(curl -s -X DELETE "$USER_API/functions/$FUNCTION_ID")
    
    if echo "$delete_response" | grep -q "Function deleted successfully"; then
        log_success "Function deletion successful"
    else
        log_error "Function deletion failed: $delete_response"
        return 1
    fi
}

# Main test execution
main() {
    log_info "Starting comprehensive Lambda@Home test suite..."
    log_info "Test function: $TEST_FUNCTION_NAME"
    log_info "User API: $USER_API"
    log_info "RIC API: $RIC_API"
    echo
    
    # Run tests
    test_health_check
    test_function_deployment
    test_container_startup
    test_single_invocation
    test_concurrent_invocations
    test_container_reuse
    test_container_scaling
    test_high_concurrency
    test_error_handling
    test_function_deletion
    
    # Print summary
    echo
    log_info "Test Summary:"
    log_success "Tests passed: $TESTS_PASSED"
    if [ "$TESTS_FAILED" -gt 0 ]; then
        log_error "Tests failed: $TESTS_FAILED"
    else
        log_success "Tests failed: $TESTS_FAILED"
    fi
    
    if [ "$TESTS_FAILED" -eq 0 ]; then
        log_success "All tests passed! Lambda@Home is working correctly."
        exit 0
    else
        log_error "Some tests failed. Please check the logs above."
        exit 1
    fi
}

# Run main function
main "$@"
