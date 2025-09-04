#!/usr/bin/env bash
echo "[legacy] Use scripts/test-service.sh for updated API tests." >&2
exec "$(dirname "$0")/../test-service.sh"
