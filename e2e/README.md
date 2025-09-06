# Lambda@Home End-to-End Tests

Professional Jest-based end-to-end test suite for Lambda@Home, providing comprehensive integration testing, performance benchmarking, and reliability validation.

## 🚀 Quick Start

1. **Install Dependencies**:
   ```bash
   cd e2e
   npm install
   ```

2. **Start Lambda@Home Server**:
   ```bash
   # From project root
   ./target/release/lambda-at-home-server &
   ```

3. **Run Tests**:
   ```bash
   npm test
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

### All Tests
```bash
npm test
```

### Specific Test Suites
```bash
# Service tests
npm run test:service

# Metrics and performance tests
npm run test:metrics

# Runtime compatibility tests
npm run test:runtimes
```

### Test Modes
```bash
# Watch mode (re-runs on file changes)
npm run test:watch

# Coverage report
npm run test:coverage

# Verbose output
VERBOSE_TESTS=1 npm test
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

**Lambda@Home E2E Tests** - Professional end-to-end testing for your local Lambda environment! 🚀
