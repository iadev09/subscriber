#!/usr/bin/env bash

CHANNEL_NAME="test-channel"

# Construct the JSON payload
payload=$(
	cat << EOF
{
  "event": "env.shutdown",
  "data": {
    "services": ["envy","evolution-session","subscriber"]
  },
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
)

# Publish the event to the Redis channel
redis-cli PUBLISH "$CHANNEL_NAME" "$payload"
