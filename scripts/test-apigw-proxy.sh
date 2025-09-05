#!/usr/bin/env bash
set -euo pipefail

# Simple API Gateway proxy-mode smoke test
# - Creates a Node.js function that returns {statusCode, headers, body}
# - Invokes it via path-based proxy: /<function-name>?q=1

USER_API=${USER_API:-http://127.0.0.1:9000}
FUNC=${FUNC:-apigw-proxy-$(date +%s)}
RUNTIME=${RUNTIME:-nodejs18.x}

tmpdir=$(mktemp -d)
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

echo "[info] Building function package for $FUNC ($RUNTIME)"
mkdir -p "$tmpdir/pkg"
cat > "$tmpdir/pkg/index.js" <<'JS'
exports.handler = async (event) => {
  // Demonstrate echoing method, path, headers, query, and body
  const result = {
    ok: true,
    method: event.httpMethod,
    path: event.path,
    query: event.queryStringParameters,
    headers: event.headers,
    body: event.body,
  };
  return {
    statusCode: 201,
    headers: { 'X-From-Lambda': 'true', 'Content-Type': 'application/json' },
    body: JSON.stringify(result),
  };
};
JS
(cd "$tmpdir/pkg" && zip -q -r "$tmpdir/$FUNC.zip" . >/dev/null)

echo "[info] Creating function $FUNC"
ZIP_B64=$(base64 < "$tmpdir/$FUNC.zip" | tr -d '\n')
cat > "$tmpdir/$FUNC.json" <<JSON
{
  "function_name": "$FUNC",
  "runtime": "$RUNTIME",
  "handler": "index.handler",
  "code": { "zip_file": "$ZIP_B64" }
}
JSON
resp=$(curl -sS -w "\n%{http_code}" -X POST "$USER_API/2015-03-31/functions" \
  -H 'content-type: application/json' \
  --data-binary @"$tmpdir/$FUNC.json")
body=$(printf "%s" "$resp" | sed '$d')
code=$(printf "%s" "$resp" | tail -n1)
if [ "$code" != "200" ]; then
  echo "[error] Create function failed (HTTP $code): $body" >&2
  exit 1
fi

sleep 1

echo "[info] Invoking via API Gateway proxy: /$FUNC?hello=world"
resp=$(curl -sS -D "$tmpdir/headers.txt" -o "$tmpdir/body.txt" \
  -H 'X-Test-Header: 123' \
  -X POST "$USER_API/$FUNC?hello=world" \
  --data-binary 'proxy-body')

status=$(head -n1 "$tmpdir/headers.txt" | awk '{print $2}')
echo "[info] HTTP status: $status"
echo "[info] Response headers:"
grep -i '^x-from-lambda:' "$tmpdir/headers.txt" || true
echo "[info] Body:"
cat "$tmpdir/body.txt"
echo

if [ "$status" != "201" ]; then
  echo "[error] Expected status 201, got $status" >&2
  exit 1
fi

echo "[ok] API Gateway proxy test succeeded"

