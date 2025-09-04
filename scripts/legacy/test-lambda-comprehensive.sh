#!/usr/bin/env bash
echo "[legacy] This script has been superseded. Use scripts/test-service.sh and scripts/test-autoscaling.sh." >&2
exec "$(dirname "$0")/../test-service.sh"
