#!/bin/bash

PORT=8080

set -e

echo "🧪 Simple Nebulet Smoke Test"
echo "============================"

echo "🏥 Checking server health..."
HEALTH_RESPONSE=$(curl -s http://localhost:$PORT/v1/health)
echo "Health response: $HEALTH_RESPONSE"

if [ $? -ne 0 ]; then
    echo "❌ Server is not responding"
    exit 1
fi

echo "✅ Server is healthy"

# Generate unique container name
CONTAINER_NAME="smoke-test-$(date +%s)"

echo "📦 Creating container: $CONTAINER_NAME"

# Create container
CREATE_RESPONSE=$(curl -s -X POST http://localhost:$PORT/v1/containers \
  -H "Content-Type: application/json" \
  -d "{\"name\": \"$CONTAINER_NAME\", \"image\": \"hello-world:latest\"}")

echo "Create response: $CREATE_RESPONSE"

# Extract container ID
CONTAINER_ID=$(echo $CREATE_RESPONSE | jq -r '.id')

if [ "$CONTAINER_ID" = "null" ] || [ -z "$CONTAINER_ID" ]; then
    echo "❌ Failed to create container"
    echo $CREATE_RESPONSE
    exit 1
fi

echo "✅ Container created with ID: $CONTAINER_ID"

# Wait a bit for container to process
echo "⏳ Waiting for container to process..."
sleep 10

# Check container status
echo "📋 Checking container status..."
CONTAINER_INFO=$(curl -s http://localhost:$PORT/v1/containers/$CONTAINER_ID)
CURRENT_STATUS=$(echo $CONTAINER_INFO | jq -r '.status')
echo "Current status: $CURRENT_STATUS"

# Wait a bit more for container to complete
echo "⏳ Waiting for container to complete..."
sleep 10

# Check final status
echo "📋 Checking final status..."
CONTAINER_INFO=$(curl -s http://localhost:$PORT/v1/containers/$CONTAINER_ID)
FINAL_STATUS=$(echo $CONTAINER_INFO | jq -r '.status')
echo "Final status: $FINAL_STATUS"

# Delete container
echo "🗑️ Deleting container..."
        DELETE_RESPONSE=$(curl -s -X DELETE http://localhost:$PORT/v1/containers/$CONTAINER_ID)
echo "Delete response: $DELETE_RESPONSE"

echo "✅ Container deleted"

echo ""
echo "🎉 Simple smoke test completed!" 