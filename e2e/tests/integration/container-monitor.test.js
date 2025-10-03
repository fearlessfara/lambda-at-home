/**
 * Container Monitor E2E Tests
 *
 * These tests verify that the container monitor properly handles bidirectional
 * state synchronization between Docker and Lambda@Home's internal state.
 *
 * Test scenarios:
 * 1. Manual container stop detection
 * 2. Container crash detection
 * 3. Container removal detection
 * 4. State synchronization after manual operations
 */

const { describe, test, before, after } = require('node:test');
const assert = require('node:assert');
const TestClient = require('../utils/test-client');
const fs = require('fs');
const path = require('path');

describe('Container Monitor Bidirectional Sync Tests', () => {
    let client;
    let testFunctionName;

    before(async () => {
        client = new TestClient();
        testFunctionName = `container-monitor-test-${Date.now()}`;
    });

    after(async () => {
        if (client) {
            try {
                await client.deleteFunction(testFunctionName);
                console.log(`✅ Cleaned up container monitor test function: ${testFunctionName}`);
            } catch (error) {
                console.error(`❌ Failed to cleanup container monitor test function ${testFunctionName}: ${error.message}`);
            }
            client.close();
        }
    });

    test('should detect manual container stop and update warm pool state', async () => {
        // Load test function ZIP
        const zipPath = path.join(__dirname, '../../test-functions/simple-test.zip');
        const zipData = fs.readFileSync(zipPath, 'base64');

        // Create function
        const createResult = await client.createFunction(
            testFunctionName,
            'nodejs18.x',
            'index.handler',
            zipData
        );
        assert.strictEqual(createResult.function_name, testFunctionName);

        // Wait for function to be ready
        await new Promise(resolve => setTimeout(resolve, 5000));

        // Get initial warm pool status
        const initialWarmPool = await client.getWarmPool(testFunctionName);
        console.log('Initial warm pool:', initialWarmPool);

        // Verify we have at least one container
        assert.ok(initialWarmPool.entries !== undefined);
        assert.ok(initialWarmPool.entries.length > 0);

        // Get the first container ID
        const containerId = initialWarmPool.entries[0].container_id;
        console.log('Testing with container:', containerId);

        // Manually stop the container using Docker
        const { execSync } = require('child_process');
        try {
            execSync(`docker stop ${containerId}`, { stdio: 'pipe' });
            console.log(`✅ Manually stopped container: ${containerId}`);
        } catch (error) {
            console.warn(`Failed to manually stop container: ${error.message}`);
            // Continue with test even if manual stop fails
        }

        // Wait for container monitor to detect the change (10 seconds + buffer)
        console.log('⏳ Waiting for container monitor to detect stop...');
        await new Promise(resolve => setTimeout(resolve, 15000));

        // Check warm pool status after manual stop
        const warmPoolAfterStop = await client.getWarmPool(testFunctionName);
        console.log('Warm pool after manual stop:', warmPoolAfterStop);

        // Verify the stopped container is no longer in the warm pool or marked as stopped
        const stoppedContainer = warmPoolAfterStop.entries.find(c => c.container_id === containerId);
        if (stoppedContainer) {
            // If container is still in warm pool, it should be marked as stopped
            assert.strictEqual(stoppedContainer.state, 'Stopped');
        } else {
            // Container should be removed from warm pool
            console.log('✅ Container was removed from warm pool after manual stop');
        }

        // Invoke function to trigger container creation
        const invokeResult = await client.invokeFunction(testFunctionName, { test: 'data' });
        assert.ok(invokeResult !== undefined);

        // Wait a moment for container to be created
        await new Promise(resolve => setTimeout(resolve, 5000));

        // Check warm pool status after invocation
        const warmPoolAfterInvoke = await client.getWarmPool(testFunctionName);
        console.log('Warm pool after invoke:', warmPoolAfterInvoke);

        // Should have at least one container (newly created)
        assert.ok(warmPoolAfterInvoke.entries.length > 0);

    }, 90000); // Increased timeout to 90 seconds
});
