#!/bin/bash

# Fitbit API Test Script
# Usage: Set your ACCESS_TOKEN and ENDPOINT, then run the script

# Configuration
ACCESS_TOKEN="eyJhbGciOiJIUzI1NiJ9.eyJhdWQiOiIyM1BEWE4iLCJzdWIiOiJCWjNKWVciLCJpc3MiOiJGaXRiaXQiLCJ0eXAiOiJhY2Nlc3NfdG9rZW4iLCJzY29wZXMiOiJyYWN0IHJzbGUiLCJleHAiOjE3NjAzNjk5OTksImlhdCI6MTc2MDM0MTE5OX0.u-IWcnw1gqEMZDV8qz9OKcWeSrSaNNqBiUfrYP84I_g"
BASE_URL="https://api.fitbit.com"

# ENDPOINT="/1/user/-/activities/date/2025-10-13.json"
ENDPOINT="/1/user/-/activities/list.json?afterDate=2025-10-13&sort=asc&offset=0&limit=5"

# Display request info
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📡 Fitbit API Request"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Method:   GET"
echo "Endpoint: ${BASE_URL}${ENDPOINT}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

# Make the API request and pretty-print the JSON
curl -s -X GET "${BASE_URL}${ENDPOINT}" \
  -H "Authorization: Bearer ${ACCESS_TOKEN}" \
  -H "Accept: application/json" | jq '.'

echo ""
