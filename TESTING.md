# Testing Lambda@Home

This project has two kinds of tests:

- Fast, local unit/integration tests run by `cargo test`.
- Smoke tests that exercise the full server + Docker lifecycle in scripts/.

Prerequisites for smoke tests:
- Docker daemon running locally
- Server running in another terminal: `make run`

## Unit and crate tests

Run all Rust tests:

```
make test
```

The control-plane autoscaling logic also includes pure decision tests in:
- `crates/control/src/autoscaler.rs` (tests for `plan_scale`)

## Smoke tests (scripts/)

See `scripts/README.md` for details. Typical flows:

### Autoscaling + reuse (continuous)

```
make run                         # in one terminal
USER_API=http://127.0.0.1:9000 \
CONCURRENCY=8 CYCLES=5          \
./scripts/test-autoscaling.sh
```

What it checks:
- Parallel burst triggers scale-out to >= MIN_BURST containers (default 2)
- Subsequent sequential invokes do not increase container count (reuse)

Tunables (env vars):
- `CONCURRENCY` (burst size, default 8)
- `SEQ_N` (sequential invokes per cycle, default 5)
- `CYCLES` (0=infinite)
- `MIN_BURST` (expectation for scale out, default 2)
- `ZIP_PATH`, `RUNTIME`, `HANDLER`, `FN_NAME`

### End-to-end service check

```
./scripts/test-service.sh
```

Runs a simple function end-to-end and checks server health and logs.

### Metrics

```
./scripts/test-metrics.sh
```

Fetches `/metrics` and validates the endpoint is reachable.

## Configuration knobs for lifecycle

Tune in `configs/default.toml`:

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

