/**
 * Custom Assertions - Helper assertions for Lambda@Home tests using Node's native assert
 */

const assert = require('node:assert');

/**
 * Assert that the response is a valid Lambda response
 */
function assertValidLambdaResponse(received) {
    assert.strictEqual(typeof received, 'object', 'Response should be an object');
    assert.strictEqual(received.success, true, 'Response should have success=true');
    assert.strictEqual(typeof received.testId, 'string', 'Response should have testId as string');
    assert.strictEqual(typeof received.message, 'string', 'Response should have message as string');
    assert.strictEqual(typeof received.timestamp, 'string', 'Response should have timestamp as string');
    assert.strictEqual(typeof received.nodeVersion, 'string', 'Response should have nodeVersion as string');
    assert.strictEqual(received.runtime, 'node', 'Response should have runtime=node');
}

/**
 * Assert that duration is within performance threshold
 */
function assertWithinPerformanceThreshold(received, threshold) {
    assert.ok(
        received <= threshold,
        `Expected ${received}ms to be within ${threshold}ms threshold`
    );
}

/**
 * Assert that results have expected number of successful invocations
 */
function assertSuccessfulInvocations(results, expectedCount) {
    const successCount = results.filter(r => r.result && r.result.success && !r.error).length;
    assert.strictEqual(
        successCount,
        expectedCount,
        `Expected ${expectedCount} successful invocations but got ${successCount}`
    );
}

/**
 * Assert that value matches object structure
 */
function assertMatchObject(received, expected) {
    for (const [key, value] of Object.entries(expected)) {
        assert.ok(
            key in received,
            `Expected property ${key} to exist in received object`
        );
        if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
            assertMatchObject(received[key], value);
        } else {
            assert.deepStrictEqual(
                received[key],
                value,
                `Expected property ${key} to match`
            );
        }
    }
}

/**
 * Assert that container count matches expected value
 */
async function assertContainerCount(functionName, expectedCount, DockerUtils) {
    const actualCount = DockerUtils.getContainerCount(functionName);
    assert.strictEqual(
        actualCount,
        expectedCount,
        `Expected ${expectedCount} container(s) for ${functionName}, but found ${actualCount}`
    );
    return actualCount;
}

/**
 * Assert that container count is within a range
 */
async function assertContainerCountInRange(functionName, minCount, maxCount, DockerUtils) {
    const actualCount = DockerUtils.getContainerCount(functionName);
    assert.ok(
        actualCount >= minCount && actualCount <= maxCount,
        `Expected ${minCount}-${maxCount} container(s) for ${functionName}, but found ${actualCount}`
    );
    return actualCount;
}

/**
 * Assert that containers are eventually cleaned up
 */
async function assertContainersCleanedUp(functionName, timeoutMs = 30000, DockerUtils) {
    const startTime = Date.now();
    let containerCount = DockerUtils.getContainerCount(functionName);

    while (containerCount > 0 && Date.now() - startTime < timeoutMs) {
        await new Promise(resolve => setTimeout(resolve, 2000));
        containerCount = DockerUtils.getContainerCount(functionName);
    }

    assert.strictEqual(
        containerCount,
        0,
        `Expected all containers for ${functionName} to be cleaned up, but found ${containerCount} after ${timeoutMs}ms`
    );

    return true;
}

module.exports = {
    assertValidLambdaResponse,
    assertWithinPerformanceThreshold,
    assertSuccessfulInvocations,
    assertMatchObject,
    assertContainerCount,
    assertContainerCountInRange,
    assertContainersCleanedUp
};
