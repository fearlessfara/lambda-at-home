# Scripts

Curated scripts for local development. These assume the Lambda@Home server is running and Docker is available.

- `test-autoscaling.sh`: Fires concurrent then sequential invocations to assert autoscaling and container reuse.
- `test-service.sh`: End-to-end sanity check of the service and a sample function container.
- `test-metrics.sh`: Quick check of the Prometheus `/metrics` endpoint.

Legacy (moved to `scripts/legacy/`): older ad-hoc scripts that may not reflect the current API or runtime behavior. Prefer the curated scripts above.

Environment variables common to tests:
- `USER_API` (default `http://127.0.0.1:9000`)
- `CONCURRENCY` (burst size for autoscaling)
- `SEQ_N` (sequential invocations per cycle)
- `ZIP_PATH`, `RUNTIME`, `HANDLER` (function packaging)

Usage:

```
make run                # start server
make test-autoscaling   # run autoscaling/reuse test
make test-service       # run end-to-end service check
```

