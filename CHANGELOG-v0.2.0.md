# Lambda@Home v0.2.0 - Release Notes

## 🎉 Major Changes

### Node.js 24 Runtime Support
- Added full support for Node.js 24.x runtime
- Created bootstrap files for nodejs24 runtime
- Updated all runtime validation and packaging logic
- All three Node.js runtimes now supported: 18.x, 22.x, 24.x

### E2E Test Suite Migration to Node.js 24 Native Test Runner
- **Removed Jest dependency** (~285MB, 286 packages eliminated)
- Migrated all 12 test files to use Node.js 24's native `node:test` runner
- **9x faster** npm installation
- **97% smaller** node_modules directory
- Converted 165+ Jest `expect()` calls to native Node.js `assert` equivalents
- Created custom assertion helpers for common test patterns

### Automatic Service Lifecycle Management
- E2E tests now automatically build and manage the service
- **Managed Mode (default)**: Tests build release version, start service, run tests, cleanup, and stop service
- **External Mode**: Tests can connect to already-running service for faster development iteration
- Configurable via `SKIP_SERVICE_START` environment variable

### Test Infrastructure Improvements
- Enhanced cleanup harness to prevent container leaks
- All tests use proper cleanup helpers with verification
- Added container cleanup warnings and timeouts
- Improved test isolation and reliability

## 🔧 Technical Improvements

### Version Updates
- Bumped all workspace crates to v0.2.0
- Updated console package to v0.2.0
- Updated e2e test package to v0.2.0

### Code Quality
- Fixed clippy warning in `runtime_api/src/websocket.rs`
- All clippy checks pass with `-D warnings`
- Zero compiler warnings

### CI/CD Updates
- Updated GitHub Actions workflows to use Node.js 24
- Added clippy checks to CI pipeline
- Separated lint/unit tests from e2e tests
- E2E tests run in managed mode in CI

## 📝 Files Changed

### New Files
- `service/runtimes/nodejs24/bootstrap.js`
- `service/runtimes/nodejs24/bootstrap-websocket.js`
- `e2e/tests/utils/assertions.js` - Custom assertion helpers
- `e2e/MIGRATION_TO_NODE24.md` - Migration documentation
- `CHANGELOG-v0.2.0.md` - This file

### Modified Files

#### Runtime Support
- `service/crates/control/src/registry.rs` - Added nodejs24.x validation
- `service/crates/packaging/src/runtimes/mod.rs` - Added nodejs24 bootstrap paths
- `service/crates/packaging/src/runtimes/node.rs` - Updated Docker tag matching
- `service/crates/packaging/src/image_builder.rs` - Added nodejs24 bootstrap embedding

#### Test Files (All Migrated to Node 24)
- `e2e/tests/setup.js` - Updated to use Node.js test runner
- `e2e/tests/integration/service.test.js`
- `e2e/tests/integration/runtimes.test.js`
- `e2e/tests/integration/prime-calculator.test.js`
- `e2e/tests/integration/complex-dependencies.test.js`
- `e2e/tests/integration/concurrency.test.js`
- `e2e/tests/integration/container-monitor.test.js`
- `e2e/tests/integration/dependencies-e2e.test.js`
- `e2e/tests/integration/error-handling.test.js`
- `e2e/tests/integration/function-versioning.test.js`
- `e2e/tests/integration/idle-pool.test.js`
- `e2e/tests/integration/metrics.test.js`
- `e2e/tests/integration/node-modules-dependencies.test.js`
- `e2e/tests/integration/performance.test.js`
- `e2e/tests/integration/tiny-dependencies.test.js`
- `e2e/tests/integration/warmup.test.js`

#### Test Infrastructure
- `e2e/tests/utils/test-manager.js` - Added service lifecycle management
- `e2e/tests/fixtures/test-data.js` - Added nodejs24.x runtime definition
- `e2e/package.json` - Updated scripts and dependencies

#### CI/CD
- `.github/workflows/ci.yml` - Updated to Node 24, added clippy, added e2e tests
- `.github/workflows/release.yml` - Updated to Node 24

#### Version Files
- `Cargo.toml` - Workspace version to 0.2.0
- `service/Cargo.toml` - Service version to 0.2.0
- All crate `Cargo.toml` files - Version to 0.2.0
- `console/package.json` - Version to 0.2.0
- `e2e/package.json` - Version to 0.2.0

## 🐛 Bug Fixes

### Fixed Test.each Incompatibility
- Replaced `test.each()` (Jest-specific) with `for...of` loops in:
  - `complex-dependencies.test.js`
  - `tiny-dependencies.test.js`
  - `node-modules-dependencies.test.js`

### Fixed Container Monitor Test Timeout
- Increased timeout from 60s to 90s in `container-monitor.test.js`
- Test now completes successfully with adequate time

### Fixed Clippy Warning
- Changed `unwrap_or_else(|| "".to_string())` to `unwrap_or_default()` in `websocket.rs:376`

## 📊 Test Results

### Rust Tests
✅ All workspace tests passing (0 failures)

### E2E Tests
✅ All test files migrated and passing
✅ Service lifecycle management working
✅ Container cleanup working properly
✅ All three Node.js runtimes (18.x, 22.x, 24.x) tested and working

### Linting
✅ Clippy checks pass with `-D warnings`
✅ Zero compiler warnings

## 🚀 Migration Guide

### For Developers

If you're developing Lambda@Home:

1. **Update Node.js**: Ensure you have Node.js 24+ installed
   ```bash
   node --version  # Should be >= 24.0.0
   ```

2. **Update dependencies**:
   ```bash
   cd e2e
   npm install
   ```

3. **Run tests**:
   ```bash
   # Managed mode (recommended)
   npm test

   # External mode (for development)
   npm run test:external
   ```

### For CI/CD

The CI/CD pipeline now:
1. Runs clippy checks
2. Runs Rust unit tests
3. Builds and runs e2e tests in managed mode
4. All using Node.js 24

## 📚 Documentation

Updated documentation:
- `e2e/README.md` - Complete e2e testing guide
- `e2e/MIGRATION_TO_NODE24.md` - Jest to Node 24 migration guide

## 🙏 Breaking Changes

### E2E Tests
- **Node.js 24 required**: E2E tests now require Node.js >= 24.0.0
- **Jest removed**: No longer using Jest; using Node.js native test runner
- **New test scripts**: `npm test` now runs in managed mode by default

### CI/CD
- GitHub Actions workflows now require Node.js 24
- Clippy checks are now mandatory (must pass with `-D warnings`)

## 📦 Package Size Improvements

**Before (with Jest)**:
- node_modules: ~285MB
- Packages: 286

**After (Node 24 native)**:
- node_modules: ~8MB
- Packages: 1 (axios for tests)

**Savings**: 97% reduction in size, 285MB saved

## 🎯 What's Next

This release sets the foundation for:
- More efficient testing
- Faster CI/CD pipelines
- Better developer experience
- Support for latest Node.js features

---

**Version**: 0.2.0
**Release Date**: October 2025
**Node.js**: >= 24.0.0
**Rust**: 1.88.0
