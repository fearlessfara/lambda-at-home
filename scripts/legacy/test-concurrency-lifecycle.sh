#!/usr/bin/env bash
echo "[legacy] This script has been superseded. Use scripts/test-autoscaling.sh instead." >&2
exec "$(dirname "$0")/../test-autoscaling.sh"
