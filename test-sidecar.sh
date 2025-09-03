#!/bin/bash

echo "🚀 Testing Lambda Sidecar Approach"

# Build and start the services
echo "📦 Building and starting Lambda Runtime API server with socat relay..."
docker-compose -f docker-compose.sidecar.yml up --build -d

# Wait for services to be ready
echo "⏳ Waiting for services to be ready..."
sleep 10

# Test the Lambda Runtime API server directly
echo "🧪 Testing Lambda Runtime API server directly..."
curl -f http://localhost:8080/health || echo "❌ Direct connection failed"

# Test the socat relay
echo "🧪 Testing socat relay..."
curl -f http://localhost:8081/health || echo "❌ Relay connection failed"

# Check if Lambda container is running
echo "📋 Checking Lambda container status..."
docker-compose -f docker-compose.sidecar.yml ps

# Show logs
echo "📋 Showing Lambda Runtime API logs..."
docker-compose -f docker-compose.sidecar.yml logs lambda-runtime-api

echo "📋 Showing socat relay logs..."
docker-compose -f docker-compose.sidecar.yml logs socat-relay

echo "📋 Showing Lambda container logs..."
docker-compose -f docker-compose.sidecar.yml logs lambda-container

echo "✅ Sidecar approach test completed"
echo "🌐 Lambda Runtime API: http://localhost:8080"
echo "🔄 Socat Relay: http://localhost:8081"
