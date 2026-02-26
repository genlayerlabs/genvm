#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$SCRIPT_DIR/../../.."

BINARY="$ROOT_DIR/build/out/bin/genvm-modules"
if [[ ! -x "$BINARY" ]]; then
	echo "Error: genvm-modules binary not found at $BINARY" >&2
	echo "Run: nix develop .#full --command ninja -C build all/bin" >&2
	exit 1
fi

PORT="${PORT:-3999}"
HOST="${HOST:-127.0.0.1}"
MANAGER_URL="http://$HOST:$PORT"

cleanup() {
	if [[ -n "${MANAGER_PID:-}" ]]; then
		kill "$MANAGER_PID" 2>/dev/null || true
		wait "$MANAGER_PID" 2>/dev/null || true
	fi
}
trap cleanup EXIT

echo "Starting manager on $MANAGER_URL..."
"$BINARY" manager --port "$PORT" --host "$HOST" --reroute-to vTEST --die-with-parent &
MANAGER_PID=$!

# Wait for manager to become healthy
for i in $(seq 1 30); do
	if curl -sf "$MANAGER_URL/status" >/dev/null 2>&1; then
		break
	fi
	if ! kill -0 "$MANAGER_PID" 2>/dev/null; then
		echo "Error: manager process died" >&2
		exit 1
	fi
	sleep 0.1
done

if ! curl -sf "$MANAGER_URL/status" >/dev/null 2>&1; then
	echo "Error: manager did not become healthy" >&2
	exit 1
fi

echo "Manager is healthy"

# Start LLM module in user_error mode
echo "Starting LLM module (user_error mode)..."
LLM_RESP=$(curl -sf -X POST "$MANAGER_URL/module/start" \
	-H 'Content-Type: application/json' \
	-d '{"module_type": "Llm", "config": null, "user_error": true, "allow_empty_backends": true}')
echo "  LLM: $LLM_RESP"

# Start Web module in user_error mode
echo "Starting Web module (user_error mode)..."
WEB_RESP=$(curl -sf -X POST "$MANAGER_URL/module/start" \
	-H 'Content-Type: application/json' \
	-d '{"module_type": "Web", "config": null, "user_error": true}')
echo "  Web: $WEB_RESP"

# Verify status
STATUS=$(curl -sf "$MANAGER_URL/status")
echo "Status: $STATUS"

echo "Manager running with erroring modules on $MANAGER_URL (PID $MANAGER_PID)"
echo "Press Ctrl+C to stop"
wait "$MANAGER_PID"
