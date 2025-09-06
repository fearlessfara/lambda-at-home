# Testing Lambda@Home

This project has two kinds of tests:

- Fast, local unit/integration tests run by `cargo test`.
- End-to-end tests that exercise the full server + Docker lifecycle in `e2e/`.

Prerequisites for e2e tests:
- Docker daemon running locally
- Server running in another terminal: `make run`

## Unit and crate tests

Run all Rust tests:

```
make test
```

The control-plane autoscaling logic also includes pure decision tests in:
- `crates/control/src/autoscaler.rs` (tests for `plan_scale`)

## End-to-end tests (e2e/)

See `e2e/README.md` for details. The e2e test suite includes:

### Service Tests
```
make test-service
# or
cd e2e && npm run test:service
```

Tests basic service functionality, health checks, and function lifecycle.

### Runtime Tests
```
make test-node-runtimes
# or
cd e2e && npm run test:runtimes
```

Tests Node.js 18.x and 22.x runtime compatibility and performance.

### Metrics Tests
```
make test-metrics
# or
cd e2e && npm run test:metrics
```

Tests performance benchmarking, load testing, and metrics collection.

### All E2E Tests
```
make test-e2e
# or
cd e2e && npm test
```

Runs the complete test suite including:
- Service functionality
- Runtime compatibility
- Performance metrics
- Container lifecycle
- Error handling
- Concurrency and throttling
- Function versioning

## Configuration knobs for lifecycle

Tune in `service/configs/default.toml`:

```
[idle]
soft_ms = 45000   # stop WarmIdle containers after this idle time
hard_ms = 300000  # remove containers after this idle time

[limits]
max_global_concurrency = 256
```

Notes:
- The Idle Watchdog checks every ~30s.
- The autoscaler reconciles every 250ms to match queue depth.
- Containers are reused (`WarmIdle -> Active -> WarmIdle`); soft-stopped containers are restarted before creating new ones.

