#!/bin/bash

# Simple test script for lambda@home
set -e

echo "🚀 Testing Lambda@Home end-to-end functionality"

# Create data directory
mkdir -p data

# Start the server in background
echo "📡 Starting Lambda@Home server..."
cargo run --bin lambda-at-home-server &
SERVER_PID=$!

# Wait for server to start
echo "⏳ Waiting for server to start..."
sleep 5

# Test health endpoint
echo "🏥 Testing health endpoint..."
curl -f http://localhost:9000/healthz || {
    echo "❌ Health check failed"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
}

# Test metrics endpoint
echo "📊 Testing metrics endpoint..."
curl -f http://localhost:9000/metrics || {
    echo "❌ Metrics endpoint failed"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
}

# Test runtime API health
echo "🔧 Testing runtime API..."
curl -f http://localhost:9001/healthz || {
    echo "❌ Runtime API health check failed"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
}

echo "✅ All basic tests passed!"

# Clean up
echo "🧹 Cleaning up..."
kill $SERVER_PID 2>/dev/null || true
wait $SERVER_PID 2>/dev/null || true

echo "🎉 Lambda@Home basic functionality test completed successfully!"
