/**
 * Service Integration Tests
 */

const { describe, test, before, after } = require('node:test');
const assert = require('node:assert');
const testData = require('../fixtures/test-data');
const { cleanupSingleFunction } = require('../utils/test-helpers');
const {
    assertValidLambdaResponse,
    assertWithinPerformanceThreshold,
    assertSuccessfulInvocations,
    assertMatchObject
} = require('../utils/assertions');

require('../setup');

describe('Lambda@Home Service Integration Tests', () => {
    let testFunction;

    before(async () => {
        testFunction = await createTestFunction('service-test');
    });

    after(cleanupSingleFunction(() => testFunction, global.testManager.client, {
        timeout: 60000,
        verifyCleanup: true,
        forceRemoveContainers: true
    }));

    describe('Health and Metrics Endpoints', () => {
        test('should have healthy server', async () => {
            const health = await global.testManager.client.healthCheck();
            assert.strictEqual(health.healthy, true);
        });

        test('should provide metrics endpoint', async () => {
            const metrics = await global.testManager.client.getMetrics();
            assert.ok(metrics.includes('lambda_cold_starts_total'));
            assert.ok(metrics.includes('lambda_duration_ms'));
        });
    });

    describe('Function Management', () => {
        test('should create function successfully', () => {
            assert.ok(testFunction.name);
            assert.strictEqual(testFunction.data.function_name, testFunction.name);
            assert.strictEqual(testFunction.data.state, 'Active');
        });

        test('should list functions', async () => {
            const functions = await global.testManager.client.listFunctions();
            assert.ok(functions.functions);
            assert.ok(Array.isArray(functions.functions));

            const ourFunction = functions.functions.find(f => f.function_name === testFunction.name);
            assert.ok(ourFunction);
        });

        test('should get function details', async () => {
            const functionData = await global.testManager.client.getFunction(testFunction.name);
            assert.strictEqual(functionData.function_name, testFunction.name);
            assert.strictEqual(functionData.runtime, 'nodejs22.x');
            assert.strictEqual(functionData.handler, 'index.handler');
        });
    });

    describe('Function Invocation', () => {
        test('should invoke function successfully', async () => {
            const result = await invokeTestFunction(
                testFunction.name,
                'service-test',
                'Hello from service test',
                100
            );

            assertValidLambdaResponse(result);
            assert.strictEqual(result.testId, 'service-test');
            assert.strictEqual(result.message, 'Hello from service test');
            assert.strictEqual(result.waitMs, 100);
        });

        test('should handle different wait times', async () => {
            const scenarios = [
                { wait: 0, description: 'immediate' },
                { wait: 50, description: 'fast' },
                { wait: 200, description: 'medium' }
            ];

            for (const scenario of scenarios) {
                const result = await invokeTestFunction(
                    testFunction.name,
                    `wait-test-${scenario.wait}`,
                    `Testing ${scenario.description} execution`,
                    scenario.wait
                );

                assertValidLambdaResponse(result);
                assert.strictEqual(result.waitMs, scenario.wait);
            }
        });

        test('should handle concurrent invocations', async () => {
            const payloadGenerator = (index) =>
                global.testManager.generateConcurrentPayload(
                    index,
                    'concurrent-test',
                    'Concurrent test',
                    25
                );

            const results = await runConcurrentInvocations(testFunction.name, 3, payloadGenerator);

            assertSuccessfulInvocations(results, 3);

            // Check that all invocations completed within reasonable time
            const maxDuration = Math.max(...results.map(r => r.duration));
            assertWithinPerformanceThreshold(maxDuration, testData.performanceThresholds.concurrentExecution);
        });

        test('should handle sequential invocations', async () => {
            const payloadGenerator = (index) =>
                global.testManager.generateSequentialPayload(
                    index,
                    'sequential-test',
                    'Sequential test',
                    50
                );

            const results = await runSequentialInvocations(testFunction.name, 5, payloadGenerator, 100);

            assert.strictEqual(results.length, 5);

            for (const result of results) {
                assertValidLambdaResponse(result.result);
                assertWithinPerformanceThreshold(result.duration, testData.performanceThresholds.mediumExecution);
            }
        });
    });

    describe('Error Handling', () => {
        test('should handle invalid function name', async () => {
            await assert.rejects(
                async () => {
                    await global.testManager.client.invokeFunction('non-existent-function', { test: 'data' });
                }
            );
        });

        test('should handle malformed payload gracefully', async () => {
            // Test with null payload
            const result = await invokeTestFunction(
                testFunction.name,
                'malformed-test',
                'Testing malformed payload',
                0,
                { invalidField: undefined }
            );

            assertValidLambdaResponse(result);
        });
    });

    describe('Performance Characteristics', () => {
        test('should maintain consistent performance', async () => {
            const iterations = 10;
            const results = [];

            for (let i = 0; i < iterations; i++) {
                const result = await measureInvocation(
                    testFunction.name,
                    global.testManager.generateTestPayload(
                        `perf-test-${i}`,
                        `Performance test ${i}`,
                        0
                    )
                );
                results.push(result);
            }

            // Calculate statistics
            const durations = results.map(r => r.duration);
            const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
            const maxDuration = Math.max(...durations);
            const minDuration = Math.min(...durations);

            assertWithinPerformanceThreshold(avgDuration, testData.performanceThresholds.fastExecution);
            assertWithinPerformanceThreshold(maxDuration, testData.performanceThresholds.mediumExecution);
            assert.ok(minDuration >= 0);
        });
    });
});
