#!/usr/bin/env bash
# Legacy script moved from repo root. Prefer using scripts/test-service.sh and scripts/test-autoscaling.sh
exec "$(dirname "$0")/../test-service.sh"
