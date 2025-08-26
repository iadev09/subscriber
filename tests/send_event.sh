#!/usr/bin/env bash

CHANNEL_NAME="test-channel"

# Generate a random version number
RANDOM_VERSION=$(shuf -i 10000-99999 -n 1)
# Generate a random 10-character hexadecimal extension
RANDOM_EXTENSION=$(openssl rand -hex 5)

TODAY_DATE=$(date +%Y%m%d)

# Construct the version string using today's date
EVOLUTION_CLIENT_VERSION="${TODAY_DATE}.${RANDOM_VERSION}.00000-${RANDOM_EXTENSION}"

# Construct the JSON payload
payload=$(
    cat <<EOF
{
  "event": "env.updated",
  "data": {
    "key": "EVOLUTION_CLIENT_VERSION",
    "value": "$EVOLUTION_CLIENT_VERSION",
    "projects": ["net777","api","ops","servant"],
    "services": [],
    "workers": ["net777:*"]
  },
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF
)

# Publish the event to the Redis channel
redis-cli PUBLISH "$CHANNEL_NAME" "$payload"