# Lambda@Home E2E Tests

End-to-end test suite for Lambda@Home using Node.js 24's native test runner, providing comprehensive integration testing, performance benchmarking, and reliability validation.

## 🚀 Quick Start

### Managed Mode (Recommended)

The test framework automatically builds and starts the service for you:

```bash
cd e2e
npm install
npm test
```

The framework will:
1. Build the service in release mode (`cargo build --release`)
2. Start the Lambda@Home server
3. Run all tests
4. Clean up containers
5. Stop the server

### External Mode (For Development)

If you want to run tests against an already-running service:

```bash
# Terminal 1: Start the service manually
cargo run --release --bin lambda-at-home-server

# Terminal 2: Run tests
cd e2e
npm install
npm run test:external
```

## 📁 Structure

```
e2e/
├── tests/
│   ├── integration/           # Integration test suites
│   │   ├── service.test.js    # Service functionality tests
│   │   ├── runtimes.test.js   # Runtime compatibility tests
│   │   └── metrics.test.js    # Performance and metrics tests
│   ├── utils/                 # Test utilities
│   │   ├── test-client.js     # HTTP client for Lambda@Home API
│   │   ├── test-manager.js    # Test function lifecycle management
│   │   └── docker-utils.js    # Docker container utilities
│   ├── fixtures/              # Test data and configurations
│   │   └── test-data.js       # Test scenarios and thresholds
│   └── setup.js              # Jest global setup and configuration
├── utils/
│   └── cleanup.js            # Manual cleanup utility
├── test-function.js          # Unified test function
├── index.js                  # Test function entry point
├── test-function.zip         # Packaged test function
└── package.json              # Dependencies and scripts
```

## 🧪 Running Tests

### Test Modes

| Mode | Command | Description |
|------|---------|-------------|
| **Managed** | `npm test` | Builds and starts service automatically |
| **External** | `npm run test:external` | Uses existing running service |
| **Watch** | `npm run test:watch` | Watch mode (requires external service) |
| **Coverage** | `npm run test:coverage` | With coverage reporting |

### Individual Test Files

With managed service (auto-build/start/stop):
```bash
node --test tests/integration/service.test.js
```

With external service:
```bash
export SKIP_SERVICE_START=1
node --test tests/integration/service.test.js
```

### Specific Test Suites
```bash
# Service tests
npm run test:service

# Runtime compatibility tests
npm run test:runtimes

# Performance tests
npm run test:performance
```

### Environment Variables
```bash
# Show verbose test output
VERBOSE_TESTS=1 npm test

# Show server logs during tests
VERBOSE_SERVER=1 npm test

# Skip service build/start (use external service)
SKIP_SERVICE_START=1 npm test
```

## 📊 Test Coverage

### Service Integration Tests
- Health checks and metrics endpoints
- Function management (create, list, get, delete)
- Function invocation with various payloads
- Concurrent and sequential execution
- Error handling and recovery
- Performance characteristics

### Runtime Tests
- Node.js 18.x and 22.x runtime support
- Runtime-specific features and compatibility
- Performance comparison across runtimes
- Memory and resource usage validation
- Error handling scenarios

### Metrics and Performance Tests
- Execution time metrics collection
- Load testing and throughput validation
- Concurrency performance testing
- Error rate monitoring
- Resource utilization testing
- Scalability validation

## 🎯 Key Features

- **Automatic Cleanup**: Functions and containers are automatically cleaned up
- **Performance Validation**: Built-in performance thresholds and benchmarking
- **Concurrent Testing**: Multi-threaded execution validation
- **Error Handling**: Comprehensive error scenario testing
- **Clean Output**: Minimal logging unless verbose mode is enabled
- **Container Management**: Automatic Docker container cleanup

## ⚙️ Configuration

### Performance Thresholds
```javascript
performanceThresholds: {
    fastExecution: 150,      // ms
    mediumExecution: 1600,   // ms
    slowExecution: 2000,     // ms
    concurrentExecution: 2000 // ms for 5 concurrent
}
```

### Environment Variables
- `VERBOSE_TESTS=1`: Enable detailed logging
- `JEST_TIMEOUT`: Override default test timeout

## 🧹 Cleanup

### Automatic Cleanup
Tests automatically clean up:
- Created Lambda functions
- Docker containers
- HTTP connections

### Manual Cleanup
```bash
# Clean up all test functions and containers
npm run cleanup
```

## 🔧 Troubleshooting

### Common Issues

1. **Server Not Running**:
   ```bash
   curl http://127.0.0.1:9000/api/healthz
   ```

2. **Port Conflicts**:
   - Ensure ports 9000 and 9001 are available

3. **Docker Issues**:
   - Ensure Docker is running
   - Check Docker permissions

### Debug Mode
```bash
# Run with verbose output
VERBOSE_TESTS=1 npm test

# Run specific test with debugging
npm test -- --testNamePattern="Service" --verbose
```

## 🚀 CI/CD Integration

```yaml
# Example GitHub Actions
- name: Run Lambda@Home E2E Tests
  run: |
    cd e2e
    npm install
    npm test
```

## 📝 Contributing

When adding new tests:
1. Follow the existing test structure
2. Use descriptive test names
3. Include proper setup/teardown
4. Add appropriate performance thresholds
5. Ensure automatic cleanup

---

## 🎉 Node.js 24 Migration

This test suite has been migrated from Jest to Node.js 24's native test runner, offering several benefits:

### Benefits
- **No external dependencies**: Jest and related packages removed (~285MB, 286 packages)
- **Faster installation**: 9x faster npm install
- **Better performance**: Native Node.js test execution
- **Modern features**: Built-in test runner, assertions, and mocking
- **Simpler stack**: One less tool to manage

### Key Changes
- `beforeAll` → `before`
- `afterAll` → `after`
- `expect(x).toBe(y)` → `assert.strictEqual(x, y)`
- `expect(x).toEqual(y)` → `assert.deepStrictEqual(x, y)`
- Custom matchers → Custom assertion functions

See `MIGRATION_TO_NODE24.md` for detailed migration guide.

---

**Lambda@Home E2E Tests v0.2.0** - Professional end-to-end testing for your local Lambda environment! 🚀
