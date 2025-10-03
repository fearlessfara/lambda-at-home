# E2E Test Cleanup Utilities

Centralized cleanup utilities for Lambda@Home e2e tests that ensure proper cleanup of functions and containers.

## Overview

The cleanup utilities provide:

- **Automatic function cleanup** with retry logic
- **Container verification** to ensure containers are properly stopped/removed
- **Timeout handling** to prevent hanging test suites
- **Container monitoring** to track Lambda container state during tests
- **Standardized patterns** for setup/teardown across all tests

## Architecture

### Core Components

1. **CleanupManager** (`cleanup-manager.js`)
   - Core cleanup engine with retry logic and container verification
   - Tracks registered functions and their containers
   - Provides monitoring snapshots and status reports

2. **Test Helpers** (`test-helpers.js`)
   - Standardized afterAll/beforeAll wrappers
   - Pre-configured cleanup functions for common patterns
   - Container assertion utilities

3. **DockerUtils** (`docker-utils.js`)
   - Low-level Docker container inspection
   - Container counting and monitoring
   - Enhanced with additional container tracking methods

4. **TestManager Integration** (`test-manager.js`)
   - Automatic registration of functions with CleanupManager
   - Container monitoring helper methods
   - Emergency cleanup capabilities

## Usage Patterns

### Pattern 1: Array of Test Functions (Most Common)

For tests that create multiple functions:

```javascript
const { cleanupAfterAll } = require('../utils/test-helpers');

describe('My Test Suite', () => {
    let testFunctions = [];

    // ... beforeAll setup ...

    afterAll(cleanupAfterAll(testFunctions, global.testManager.client, {
        timeout: 90000,
        verifyCleanup: true,
        forceRemoveContainers: true
    }), 90000);

    test('my test', async () => {
        const func = await createTestFunction('test');
        testFunctions.push(func);
        // ... test code ...
    });
});
```

### Pattern 2: Single Test Function

For tests with one function:

```javascript
const { cleanupSingleFunction } = require('../utils/test-helpers');

describe('My Test Suite', () => {
    let testFunction;

    beforeAll(async () => {
        testFunction = await createTestFunction('my-test');
    });

    afterAll(cleanupSingleFunction(() => testFunction, global.testManager.client, {
        timeout: 60000,
        verifyCleanup: true
    }), 60000);
});
```

Note: Use `() => testFunction` (function) not `testFunction` (value) to ensure latest reference.

### Pattern 3: Cleanup with Temp Files

For tests that create temporary files (like ZIP files):

```javascript
const { cleanupWithTempFiles } = require('../utils/test-helpers');

describe('My Test Suite', () => {
    let testFunctions = [];
    let tempZipPath = null;

    beforeAll(async () => {
        tempZipPath = createTempZip(); // your function
        // ... create functions ...
    });

    afterAll(cleanupWithTempFiles(
        testFunctions,
        global.testManager.client,
        [tempZipPath], // array of file paths to delete
        {
            timeout: 90000,
            verbose: true
        }
    ), 90000);
});
```

### Pattern 4: Container Monitoring in Tests

For tests that need to verify container behavior:

```javascript
const { containerAssertions } = require('../utils/test-helpers');
const DockerUtils = require('../utils/docker-utils');

test('should clean up containers', async () => {
    const func = await createTestFunction('test');

    // Get initial container count
    const initialCount = global.testManager.getFunctionContainerCount(func.name);
    expect(initialCount).toBeGreaterThan(0);

    // Invoke function
    await invokeFunction(func.name, {});

    // Wait for container count to reach expected value
    await global.testManager.waitForContainerCount(func.name, 1, 10000);

    // Delete function
    await global.testManager.deleteFunction(func.name);

    // Assert containers are cleaned up
    await containerAssertions.assertContainersCleanedUp(func.name, 30000, DockerUtils);
});
```

## Configuration Options

All cleanup helpers accept an options object:

```javascript
{
    timeout: 90000,              // Cleanup timeout in ms (default: 60000)
    verifyCleanup: true,         // Verify containers are removed (default: true)
    forceRemoveContainers: true, // Force remove stuck containers (default: true)
    parallel: true,              // Delete functions in parallel (default: true)
    verbose: false,              // Enable detailed logging (default: false)
    throwOnFailure: false        // Throw error if cleanup fails (default: false)
}
```

## Container Monitoring

### Get Container Snapshot

```javascript
const snapshot = global.testManager.getContainerSnapshot();
console.log(snapshot);
// {
//   timestamp: '2025-10-03T...',
//   totalLambdaContainers: 5,
//   functionContainers: {
//     'my-function-123': 2,
//     'other-function-456': 3
//   },
//   allContainers: [{ name: 'lambda-...', status: 'Up 5 seconds' }, ...]
// }
```

### Get Cleanup Status

```javascript
const status = global.testManager.getCleanupStatus();
console.log(status);
// {
//   registeredFunctions: ['func-1', 'func-2'],
//   registeredCount: 2,
//   containerSnapshot: { ... },
//   cleanupConfiguration: { timeout: 60000, retries: 3, ... }
// }
```

### Container Utilities (DockerUtils)

```javascript
const DockerUtils = require('../utils/docker-utils');

// Get count of containers for a function
const count = DockerUtils.getContainerCount('my-function');

// Get all Lambda containers (running only)
const running = DockerUtils.getLambdaContainers();

// Get all Lambda containers (including exited)
const all = DockerUtils.getAllLambdaContainers(true);

// Get containers for specific function
const funcContainers = DockerUtils.getLambdaContainersByFunction('my-function', false);

// Wait for container count to reach target
await DockerUtils.waitForContainerCount('my-function', 2, 10000);
```

## Emergency Cleanup

If tests are completely broken and containers are stuck:

```javascript
// In a test or test setup:
await global.testManager.emergencyCleanup();
```

This will forcibly remove ALL Lambda containers and clear the function registry.

⚠️ **Use sparingly** - this is a nuclear option for when normal cleanup fails.

## Best Practices

### 1. Always Use Timeouts

Always specify a timeout for afterAll to prevent hanging:

```javascript
afterAll(cleanupAfterAll(testFunctions, client, { timeout: 90000 }), 90000);
//                                                                    ^^^^^^
//                                                Jest afterAll timeout
```

### 2. Enable Force Remove for Flaky Tests

If containers sometimes get stuck:

```javascript
afterAll(cleanupAfterAll(testFunctions, client, {
    forceRemoveContainers: true // Force remove any stuck containers
}), 90000);
```

### 3. Use Verbose Mode for Debugging

When debugging cleanup issues:

```javascript
afterAll(cleanupAfterAll(testFunctions, client, {
    verbose: true // Detailed logging of cleanup process
}), 90000);
```

### 4. Verify Cleanup in Critical Tests

For tests that must guarantee clean state:

```javascript
afterAll(cleanupAfterAll(testFunctions, client, {
    verifyCleanup: true,
    throwOnFailure: true // Fail test if cleanup incomplete
}), 90000);
```

### 5. Monitor Containers During Tests

For tests verifying scaling/warmup behavior:

```javascript
test('should scale containers', async () => {
    const before = global.testManager.getContainerSnapshot();

    // Trigger scaling...

    const after = global.testManager.getContainerSnapshot();
    expect(after.totalLambdaContainers).toBeGreaterThan(before.totalLambdaContainers);
});
```

## Troubleshooting

### Issue: AfterAll timeout

**Cause**: Functions taking too long to delete or containers stuck

**Solution**:
```javascript
// Increase timeout and enable force remove
afterAll(cleanupAfterAll(testFunctions, client, {
    timeout: 120000,
    forceRemoveContainers: true
}), 120000);
```

### Issue: Containers not cleaned up

**Cause**: Delete API not removing containers or containers stuck

**Solution**:
```javascript
// Enable verification and force remove
afterAll(cleanupAfterAll(testFunctions, client, {
    verifyCleanup: true,
    forceRemoveContainers: true,
    verbose: true // See what's happening
}), 90000);
```

### Issue: Test suite leaves containers running

**Cause**: Test failure before cleanup or missing afterAll

**Solution**:
1. Always use standardized cleanup helpers
2. Use try/catch in beforeAll if setup can fail
3. Run emergency cleanup between test runs if needed

```javascript
// Run before starting tests:
await global.testManager.emergencyCleanup();
```

## Migration Guide

### Old Pattern (Manual Cleanup)

```javascript
afterAll(async () => {
    for (const func of testFunctions) {
        try {
            await client.deleteFunction(func.name);
        } catch (error) {
            console.error(`Failed to delete: ${error.message}`);
        }
    }
});
```

### New Pattern (Standardized Cleanup)

```javascript
const { cleanupAfterAll } = require('../utils/test-helpers');

afterAll(cleanupAfterAll(testFunctions, client, {
    timeout: 90000,
    verifyCleanup: true,
    forceRemoveContainers: true
}), 90000);
```

## Benefits

✅ **Automatic retry logic** - Handles transient failures
✅ **Container verification** - Ensures containers are actually removed
✅ **Timeout protection** - Prevents infinite hangs
✅ **Parallel cleanup** - Faster test suite completion
✅ **Force cleanup** - Handles stuck containers
✅ **Consistent patterns** - Same approach across all tests
✅ **Container monitoring** - Track container state during tests
✅ **Better debugging** - Verbose mode for troubleshooting

## Examples

See updated test files for examples:
- `tests/integration/runtimes.test.js` - Multiple functions cleanup
- `tests/integration/service.test.js` - Single function cleanup
- `tests/integration/complex-dependencies.test.js` - Cleanup with temp files
- `tests/integration/container-monitor.test.js` - Container monitoring

## API Reference

### CleanupManager

```javascript
const cleanupManager = new CleanupManager(client, options);

// Register function for cleanup
cleanupManager.registerFunction(functionName);

// Unregister function
cleanupManager.unregisterFunction(functionName);

// Get container snapshot
const snapshot = cleanupManager.getContainerSnapshot();

// Wait for container count
await cleanupManager.waitForContainerCount(functionName, targetCount, timeoutMs);

// Cleanup all registered functions
const result = await cleanupManager.cleanup(options);

// Emergency cleanup
await cleanupManager.emergencyCleanup();
```

### Test Helpers

```javascript
const {
    cleanupAfterAll,
    cleanupSingleFunction,
    cleanupWithTempFiles,
    cleanupFunctionsByName,
    withTimeout,
    containerAssertions
} = require('../utils/test-helpers');

// See usage patterns above for examples
```
