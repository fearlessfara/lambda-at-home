# Contributing to Lambda@Home

Thanks for your interest in contributing! This project aims to provide a pragmatic, Docker‑backed Lambda experience with a clean developer workflow.

## Getting Started

Prerequisites:
- Rust (1.75+) and cargo
- Docker daemon running locally

Build and run:
```
make build
make run
```

Run tests:
```
make test            # Rust unit/integration tests
make test-service    # Service smoke test (requires server + Docker)
make test-autoscaling
```

## Code Style

- Rustfmt and Clippy should pass:
```
make fmt
make clippy
```
- Keep PRs focused and scoped.
- Prefer small modules with clear responsibilities.
- Follow existing patterns for logging and error handling.

## Tests

- Add unit tests when you add new pure logic (e.g., planners, parsers).
- Integration tests live under the crate’s `tests/` directory.
- Runtime and server smoke tests are under `scripts/`.

## Commit/PR Guidelines

- Write clear commit messages and PR descriptions.
- Include motivation, approach, and any trade‑offs.
- Reference related issues where relevant.
- If a change touches lifecycle behavior, mention expected metrics/observability changes.

## Project Areas

- Control plane (scheduler, warm pool, autoscaler)
- Runtime API (container protocol)
- Invoker (Docker integration)
- Packaging (ZIP + image build/cache)
- Metrics (Prometheus)

## Roadmap (high‑level)

- Provisioned concurrency reconciler
- Draining on deploy/version rollout
- Backoff/health for init failures; quarantining
- Per‑function reserved concurrency
- Richer metrics + dashboards
- Node/Python bootstrap hardening and tests

Thanks again — happy hacking!
