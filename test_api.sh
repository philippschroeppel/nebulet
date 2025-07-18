#!/bin/bash

# Simple API test script for Nebulet
# Usage: ./test_api.sh

BASE_URL="http://localhost:8080/v1"

echo "🧪 Testing Nebulet API at $BASE_URL"
echo "=================================="

# Helper function to check if response has error
check_error() {
    local response="$1"
    local status_code="$2"
    local test_name="$3"
    
    # Check if status code indicates error (4xx or 5xx)
    if [ "$status_code" -ge 400 ]; then
        echo "❌ $test_name failed with status $status_code: $(echo "$response" | jq -r '.error // "Unknown error"')"
        return 1
    elif echo "$response" | jq -e 'has("error")' > /dev/null 2>&1; then
        echo "❌ $test_name failed: $(echo "$response" | jq -r '.error')"
        return 1
    else
        echo "✅ $test_name passed"
        return 0
    fi
}

# Helper function to make HTTP request and capture status code
make_request() {
    local method="$1"
    local url="$2"
    local data="$3"
    
    if [ -n "$data" ]; then
        # POST/PUT request with data
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$url" \
            -H "Content-Type: application/json" \
            -d "$data")
    else
        # GET/DELETE request without data
        response=$(curl -s -w "\n%{http_code}" -X "$method" "$url")
    fi
    
    # Extract status code (last line) and response body (everything else)
    status_code=$(echo "$response" | tail -n1)
    response_body=$(echo "$response" | sed '$d')
    
    echo "$response_body"
    echo "$status_code"
}

# Test health check
echo -e "\n1️⃣  Testing health check..."
health_result=$(make_request "GET" "$BASE_URL/health")
health_response=$(echo "$health_result" | sed '$d')
health_status=$(echo "$health_result" | tail -n1)
echo "$health_response" | jq .
check_error "$health_response" "$health_status" "Health check"

# Test list containers (should be empty initially)
echo -e "\n2️⃣  Testing list containers..."
list_result=$(make_request "GET" "$BASE_URL/containers")
list_response=$(echo "$list_result" | sed '$d')
list_status=$(echo "$list_result" | tail -n1)
echo "$list_response" | jq .
check_error "$list_response" "$list_status" "List containers"

# Test create container
echo -e "\n3️⃣  Testing create container..."
create_data='{"name": "test-nginx", "image": "nginx:latest"}'
create_result=$(make_request "POST" "$BASE_URL/containers" "$create_data")
create_response=$(echo "$create_result" | sed '$d')
create_status=$(echo "$create_result" | tail -n1)
echo "$create_response" | jq .

if check_error "$create_response" "$create_status" "Create container"; then
    # Extract container ID for later tests
    CONTAINER_ID=$(echo "$create_response" | jq -r '.id')
    echo "📦 Container ID: $CONTAINER_ID"
else
    echo "❌ Cannot continue - create container failed"
    exit 1
fi

# Test get specific container
echo -e "\n4️⃣  Testing get container..."
get_result=$(make_request "GET" "$BASE_URL/containers/$CONTAINER_ID")
get_response=$(echo "$get_result" | sed '$d')
get_status=$(echo "$get_result" | tail -n1)
echo "$get_response" | jq .
check_error "$get_response" "$get_status" "Get container"

# Test list containers again (should now have one container)
echo -e "\n5️⃣  Testing list containers (should have one now)..."
list2_result=$(make_request "GET" "$BASE_URL/containers")
list2_response=$(echo "$list2_result" | sed '$d')
list2_status=$(echo "$list2_result" | tail -n1)
echo "$list2_response" | jq .
check_error "$list2_response" "$list2_status" "List containers"

# Test delete container
echo -e "\n6️⃣  Testing delete container..."
delete_result=$(make_request "DELETE" "$BASE_URL/containers/$CONTAINER_ID")
delete_response=$(echo "$delete_result" | sed '$d')
delete_status=$(echo "$delete_result" | tail -n1)
echo "$delete_response" | jq .
check_error "$delete_response" "$delete_status" "Delete container"

# Test list containers again (should be empty again)
echo -e "\n7️⃣  Testing list containers (should be empty again)..."
list3_result=$(make_request "GET" "$BASE_URL/containers")
list3_response=$(echo "$list3_result" | sed '$d')
list3_status=$(echo "$list3_result" | tail -n1)
echo "$list3_response" | jq .
check_error "$list3_response" "$list3_status" "List containers"

# Test get non-existent container (should return 404)
echo -e "\n8️⃣  Testing get non-existent container (should return 404)..."
not_found_result=$(make_request "GET" "$BASE_URL/containers/non-existent-id")
not_found_response=$(echo "$not_found_result" | sed '$d')
not_found_status=$(echo "$not_found_result" | tail -n1)
echo "$not_found_response" | jq .

# For 404, we expect an error status code
if [ "$not_found_status" -eq 404 ]; then
    echo "✅ Get non-existent container passed (returned 404 as expected)"
else
    echo "❌ Get non-existent container failed (expected 404, got $not_found_status)"
fi

echo -e "\n🎉 API test completed!"
echo "==================================" 