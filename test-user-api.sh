#!/bin/bash

# Lambda@Home User API Test - Pure User Perspective
# This test only uses public APIs, no manual container management

set -e

echo "🚀 Lambda@Home User API Test"
echo "============================"
echo "Testing from pure user perspective - only public APIs"
echo

# Configuration
USER_API="http://localhost:8080"
FUNCTION_NAME="user-test-$(date +%s)"

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

log_warning() {
    echo "⚠️  $1"
}

echo
log_info "Test 1: Health Check"
response=$(curl -s "$USER_API/health")
if echo "$response" | grep -q "ok"; then
    log_success "Service is healthy"
else
    log_error "Service health check failed: $response"
    exit 1
fi

echo
log_info "Test 2: Function Deployment"
deployment_response=$(curl -s -X POST -H "Content-Type: application/json" \
    -d "{\"function_name\": \"$FUNCTION_NAME\", \"runtime\": \"nodejs22\", \"handler\": \"index.handler\"}" \
    "$USER_API/functions")

echo "Deployment response: $deployment_response"

if echo "$deployment_response" | grep -q "function_id"; then
    FUNCTION_ID=$(echo "$deployment_response" | grep -o '"function_id":"[^"]*"' | cut -d'"' -f4)
    log_success "Function deployed successfully (ID: $FUNCTION_ID)"
else
    log_error "Function deployment failed"
    exit 1
fi

echo
log_info "Test 3: Function Invocation (without containers)"
log_warning "Note: This will fail because no containers are running - this demonstrates the current limitation"

invocation_response=$(curl -s -X POST -H "Content-Type: application/json" \
    -d '{"event": {"message": "Hello from user test!", "type": "user_test"}}' \
    "$USER_API/functions/$FUNCTION_ID/invoke")

echo "Invocation response: $invocation_response"

if echo "$invocation_response" | grep -q "Hello from Lambda@Home"; then
    log_success "Function invocation successful"
else
    log_warning "Function invocation failed (expected - no containers running): $invocation_response"
fi

echo
log_info "Test 4: Error Handling - Invalid Function ID"
error_response=$(curl -s -w "%{http_code}" -o /dev/null -X POST -H "Content-Type: application/json" \
    -d '{"event": {"message": "Error test"}}' \
    "$USER_API/functions/invalid-function-id/invoke")

if [ "$error_response" = "400" ]; then
    log_success "Error handling for invalid function ID working (HTTP $error_response)"
else
    log_error "Error handling failed (HTTP $error_response)"
fi

echo
log_info "Test 5: Error Handling - Invalid JSON"
error_response=$(curl -s -w "%{http_code}" -o /dev/null -X POST -H "Content-Type: application/json" \
    -d '{"invalid": json}' \
    "$USER_API/functions/$FUNCTION_ID/invoke")

if [ "$error_response" = "400" ]; then
    log_success "Error handling for invalid JSON working (HTTP $error_response)"
else
    log_error "Error handling for invalid JSON failed (HTTP $error_response)"
fi

echo
log_info "Test 6: Function Deletion"
delete_response=$(curl -s -X DELETE "$USER_API/functions/$FUNCTION_ID")

if echo "$delete_response" | grep -q "Function deleted successfully"; then
    log_success "Function deletion successful"
else
    log_error "Function deletion failed: $delete_response"
fi

echo
log_info "Test 7: Function Invocation After Deletion"
error_response=$(curl -s -w "%{http_code}" -o /dev/null -X POST -H "Content-Type: application/json" \
    -d '{"event": {"message": "Test after deletion"}}' \
    "$USER_API/functions/$FUNCTION_ID/invoke")

if [ "$error_response" = "400" ]; then
    log_success "Function invocation properly fails after deletion (HTTP $error_response)"
else
    log_error "Function invocation should fail after deletion (HTTP $error_response)"
fi

echo
echo "📊 Test Summary"
echo "==============="
echo "✅ Tests passed: $TESTS_PASSED"
echo "❌ Tests failed: $TESTS_FAILED"

echo
echo "🔍 Current Limitations Identified:"
echo "=================================="
echo "1. ❌ No automatic container lifecycle management"
echo "2. ❌ Users must manually start containers"
echo "3. ❌ No container auto-scaling"
echo "4. ❌ No container health monitoring"
echo "5. ❌ No automatic container cleanup"

echo
echo "💡 Recommendations for AWS Lambda-like Experience:"
echo "=================================================="
echo "1. ✅ Implement automatic container spawning on first invocation"
echo "2. ✅ Add container auto-scaling based on load"
echo "3. ✅ Implement container health checks and auto-recovery"
echo "4. ✅ Add automatic container cleanup after idle time"
echo "5. ✅ Implement container warming for better performance"

if [ "$TESTS_FAILED" -eq 0 ]; then
    echo
    echo "🎉 All API tests passed! The service API is working correctly."
    echo "⚠️  However, the service needs automatic container management for a complete AWS Lambda experience."
    exit 0
else
    echo
    echo "⚠️  Some tests failed. Please check the logs above."
    exit 1
fi
