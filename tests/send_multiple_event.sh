#!/bin/bash

SCRIPT_DIR="$(cd $(dirname "${BASH_SOURCE[0]}") && pwd)"

count=${1:-3}
delay=${2:-0}
shutdown=${3:-no}

shutdown_order=$((RANDOM % count + 1)) # [1..count]
total_subscribers=0

# Validate delay is numeric
if ! [[ "$delay" =~ ^[0-9\.]+$ ]]; then
	echo "❌ Error: delay must be a numeric value (got: '$delay')" >&2
	exit 1
fi

# Validate shutdown is 'yes' or 'no'
if [[ "$shutdown" != "yes" && "$shutdown" != "no" ]]; then
	echo "❌ Error: shutdown must be either 'yes' or 'no' (got: '$shutdown')" >&2
	exit 1
fi

echo "🧪 Starting $count event(s) with delay=$delay sec, shutdown=$shutdown, shutdown_order=$shutdown_order"

for ((i = 1; i <= count; i++)); do
	echo "⚙️  Sending 'env.updated' iteration $i"

	# Run send_event.sh and capture Redis PUBLISH result
	result=$("${SCRIPT_DIR}/send_event.sh")
	count_this=$(echo "$result" | grep -oE '[0-9]+' | head -n1)

	if [[ "$count_this" -eq 0 ]]; then
		echo "❌ no caught"
	else
		echo "✅ published to $result subscribers"
	fi

	if [[ -n "$count_this" ]]; then
		total_subscribers=$((total_subscribers + $count_this))
	fi

	# Check if this iteration is the one to trigger shutdown
	if [[ $i -eq $shutdown_order && $shutdown == "yes" ]]; then
		echo "💀 Sending SHUTDOWN  at iteration $i"
		result=$("${SCRIPT_DIR}/send_shutdown.sh")
		count_this=$(echo "$result" | grep -oE '[0-9]+' | head -n1)

		if [[ "$count_this" -eq 0 ]]; then
			echo "❌ Shutdown not caught by any subscriber"
		else
			echo "✅ Shutdown published to $count_this subscribers"
			total_subscribers=$((total_subscribers + $count_this))
		fi
	fi

	sleep "$delay"
done
echo
echo "✅ Completed $count iterations"
echo "📊 Total subscribers reached across all events: $total_subscribers"
