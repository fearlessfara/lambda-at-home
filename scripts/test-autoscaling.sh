#!/usr/bin/env bash
set -euo pipefail

# Autoscaling smoke test for Lambda@Home
# Requires: running server, Docker daemon, and test-function.zip in repo root

USER_API=${USER_API:-http://127.0.0.1:9000}
FN_NAME=${FN_NAME:-autoscale-$(date +%s)}
ZIP_PATH=${ZIP_PATH:-./test-function.zip}
RUNTIME=${RUNTIME:-nodejs18.x}
HANDLER=${HANDLER:-index.handler}
CONCURRENCY=${CONCURRENCY:-8}

echo "[info] Creating function '$FN_NAME' at $USER_API"
# Portable base64 (GNU and BSD/macOS): read via stdin and strip newlines
if [ -f "$ZIP_PATH" ]; then
  ZIP_B64=$(base64 < "$ZIP_PATH" | tr -d '\n')
else
  echo "[error] ZIP file not found: $ZIP_PATH" >&2
  exit 1
fi
CREATE_PAYLOAD=$(cat <<JSON
{
  "function_name": "$FN_NAME",
  "runtime": "$RUNTIME",
  "handler": "$HANDLER",
  "code": { "zip_file": "$ZIP_B64" }
}
JSON
)

curl -sS -X POST "$USER_API/2015-03-31/functions" \
  -H 'content-type: application/json' \
  -d "$CREATE_PAYLOAD" >/dev/null

echo "[info] Warming function with one invoke"
curl -sS -X POST "$USER_API/2015-03-31/functions/$FN_NAME/invocations" \
  -H 'content-type: application/json' \
  -d '{"ping":1}' >/dev/null || true

sleep 1


# Helpers
current_count() {
  docker ps --format '{{.Names}}' | grep -E "^lambda-$FN_NAME-" | wc -l | tr -d ' '
}

fire_parallel() {
  local n=$1
  echo "[info] Firing $n concurrent invocations"
  local pids=()
  for i in $(seq 1 "$n"); do
    curl -sS -X POST "$USER_API/2015-03-31/functions/$FN_NAME/invocations" \
      -H 'content-type: application/json' \
      -d "{\"req\":$i}" >/dev/null &
    pids+=("$!")
  done
  for p in "${pids[@]}"; do wait "$p" || true; done
}

fire_sequential() {
  local n=$1
  echo "[info] Firing $n sequential invocations (reuse expected)"
  for i in $(seq 1 "$n"); do
    curl -sS -X POST "$USER_API/2015-03-31/functions/$FN_NAME/invocations" \
      -H 'content-type: application/json' \
      -d "{\"seq\":$i}" >/dev/null || true
    sleep 0.5
  done
}

poll_until_ge() {
  local target=$1
  local timeout=${2:-10}
  local waited=0
  while [ "$waited" -lt "$timeout" ]; do
    local c; c=$(current_count)
    if [ "$c" -ge "$target" ]; then
      echo "$c"
      return 0
    fi
    sleep 1; waited=$((waited+1))
  done
  current_count
}

CYCLES=${CYCLES:-0} # 0 = infinite
SEQ_N=${SEQ_N:-5}
MIN_BURST=${MIN_BURST:-2}

echo "[info] Starting autoscaling cycles (CYCLES=${CYCLES:-inf}, CONCURRENCY=$CONCURRENCY, SEQ_N=$SEQ_N) — Ctrl+C to stop"

cycle=1
while :; do
  if [ "$CYCLES" -ne 0 ] && [ "$cycle" -gt "$CYCLES" ]; then break; fi
  echo "[cycle $cycle] Parallel burst"
  fire_parallel "$CONCURRENCY"
  burst_count=$(poll_until_ge "$MIN_BURST" 12)
  echo "[cycle $cycle] Containers after burst: $burst_count"
  if [ "$burst_count" -lt "$MIN_BURST" ]; then
    echo "[error] Expected >= $MIN_BURST containers after burst, saw $burst_count" >&2
    exit 1
  fi

  echo "[cycle $cycle] Sequential invokes to assert reuse"
  fire_sequential "$SEQ_N"
  post_seq_count=$(current_count)
  echo "[cycle $cycle] Containers after sequential: $post_seq_count"
  if [ "$post_seq_count" -gt "$burst_count" ]; then
    echo "[error] Container count increased during sequential reuse (from $burst_count to $post_seq_count)" >&2
    exit 1
  fi

  echo "[cycle $cycle] OK — reuse asserted (no new containers)"
  cycle=$((cycle+1))
  sleep 2
done

echo "[ok] Autoscaling + reuse test completed"
