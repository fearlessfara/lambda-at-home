#!/usr/bin/env bash
set -euo pipefail

# Tests both Node.js 18.x and 22.x function runners by creating
# one function per runtime, invoking each, and printing the result.

USER_API=${USER_API:-http://127.0.0.1:9000}
FN18=${FN18:-node18-$(date +%s)}
FN22=${FN22:-node22-$(date +%s)}

tmpdir=$(mktemp -d)
cleanup() { rm -rf "$tmpdir"; }
trap cleanup EXIT

make_zip() {
  local out_zip=$1
  local msg=$2
  mkdir -p "$tmpdir/pkg"
  cat > "$tmpdir/pkg/index.js" <<'JS'
exports.handler = async (event) => {
  return {
    ok: true,
    event,
    nodeVersion: process.version,
    runtime: 'node',
  };
};
JS
  (cd "$tmpdir/pkg" && zip -q -r "$out_zip" . >/dev/null)
}

create_function() {
  local name=$1
  local runtime=$2
  local zip=$3
  local b64
  b64=$(base64 < "$zip" | tr -d '\n')
  local payload="$tmpdir/$name.json"
  cat > "$payload" <<JSON
{
  "function_name": "$name",
  "runtime": "$runtime",
  "handler": "index.handler",
  "code": { "zip_file": "$b64" }
}
JSON
  resp=$(curl -sS -w "\n%{http_code}" -X POST "$USER_API/2015-03-31/functions" \
    -H 'content-type: application/json' \
    --data-binary @"$payload")
  body=$(printf "%s" "$resp" | sed '$d')
  code=$(printf "%s" "$resp" | tail -n1)
  if [ "$code" != "200" ]; then
    echo "[error] Create function $name failed (HTTP $code): $body" >&2
    exit 1
  fi
  echo "[info] Created $name ($runtime)"
}

invoke_function() {
  local name=$1
  curl -sS -X POST "$USER_API/2015-03-31/functions/$name/invocations" \
    -H 'content-type: application/json' \
    -d '{"hello":"world"}'
}

echo "[info] Creating test packages"
ZIP18="$tmpdir/node18.zip"
ZIP22="$tmpdir/node22.zip"
make_zip "$ZIP18" "node18"
make_zip "$ZIP22" "node22"

echo "[info] Creating functions: $FN18 (nodejs18.x), $FN22 (nodejs22.x)"
create_function "$FN18" nodejs18.x "$ZIP18"
create_function "$FN22" nodejs22.x "$ZIP22"

echo "[info] Verifying creation and invoking $FN18"
curl -sS "$USER_API/2015-03-31/functions/$FN18" >/dev/null || { echo "[error] Function $FN18 not found after creation" >&2; exit 1; }
RESP18=$(invoke_function "$FN18")
echo "$RESP18"

echo "[info] Verifying creation and invoking $FN22"
curl -sS "$USER_API/2015-03-31/functions/$FN22" >/dev/null || { echo "[error] Function $FN22 not found after creation" >&2; exit 1; }
RESP22=$(invoke_function "$FN22")
echo "$RESP22"

echo "[ok] Node runtimes test complete"
