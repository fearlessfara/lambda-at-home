# Contributing to Lambda@Home

Thanks for your interest in contributing! This project aims to provide a pragmatic, Docker‑backed Lambda experience with a clean developer workflow.

## Getting Started

Prerequisites:
- Rust (1.75+) and cargo
- Docker daemon running locally
 - Node.js 18+ (for the web console)

Build and run the server (embeds the console assets):
```
make build      # builds console + Rust workspace
make run
```

Run tests:
```
make test             # Rust unit tests (workspace)
make test-service     # E2E service smoke tests (requires server + Docker)
make test-metrics     # E2E metrics checks
make test-node-runtimes  # E2E Node 18.x / 22.x
make test-e2e         # Run all E2E tests (see e2e/)
```

## Code Style

- Rustfmt and Clippy should pass:
```
make fmt
make clippy
```
- Console lint (optional during UI work):
```
cd console && npm run lint
```
- Keep PRs focused and scoped.
- Prefer small modules with clear responsibilities.
- Follow existing patterns for logging and error handling.

## Tests

- Add unit tests when you add new pure logic (e.g., planners, parsers).
- Crate integration tests live under each crate’s `tests/` directory.
- End‑to‑end tests live in `e2e/` (Node/Jest). Start the server with `make run` in one terminal, then run E2E in another.

Repository layout (high level):

```
service/         # Rust workspace (API, runtime_api, control, invoker, packaging, models, metrics)
console/         # Vite/React UI embedded into the server binary
e2e/             # End‑to‑end tests (Jest)
examples/        # Example functions
```

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
 - Web console (React + Vite)

## Roadmap (high‑level)

- Provisioned concurrency reconciler
- Draining on deploy/version rollout
- Backoff/health for init failures; quarantining
- Per‑function reserved concurrency
- Richer metrics + dashboards
- Node/Python bootstrap hardening and tests

Thanks again — happy hacking!
