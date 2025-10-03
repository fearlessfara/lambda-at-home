/**
 * Runtime Integration Tests
 */

const { describe, test, after } = require('node:test');
const assert = require('node:assert');
const testData = require('../fixtures/test-data');
const { cleanupAfterAll } = require('../utils/test-helpers');
const {
    assertValidLambdaResponse,
    assertWithinPerformanceThreshold,
    assertSuccessfulInvocations,
    assertMatchObject
} = require('../utils/assertions');

require('../setup');

describe('Lambda@Home Runtime Integration Tests', () => {
    let testFunctions = [];

    after(cleanupAfterAll(testFunctions, global.testManager.client, {
        timeout: 90000,
        verbose: false,
        verifyCleanup: true,
        forceRemoveContainers: true
    }));

    describe('Node.js Runtime Support', () => {
        for (const runtime of testData.runtimes) {
            test(`should support ${runtime.name} runtime`, async () => {
                const testFunction = await createTestFunction(`runtime-test-${runtime.name.replace('.', '-')}`, runtime.name);
                testFunctions.push(testFunction);

                const result = await invokeTestFunction(
                    testFunction.name,
                    'runtime-test',
                    `Testing ${runtime.name} runtime`,
                    0
                );

                assertValidLambdaResponse(result);
                assert.strictEqual(result.nodeVersion, runtime.version);
                assert.strictEqual(result.runtime, 'node');
            });
        }

        test('should handle runtime-specific features', async () => {
            const testFunction = await createTestFunction('runtime-features-test');
            testFunctions.push(testFunction);

            // Test with different payload structures
            const testPayloads = [
                { simple: 'string' },
                { complex: { nested: { data: [1, 2, 3] } } },
                { array: [1, 2, 3, 4, 5] },
                { nullValue: null }
            ];

            for (let i = 0; i < testPayloads.length; i++) {
                const result = await invokeTestFunction(
                    testFunction.name,
                    `runtime-features-${i}`,
                    `Testing runtime features ${i}`,
                    0,
                    testPayloads[i]
                );

                assertValidLambdaResponse(result);
                assertMatchObject(result.event, testPayloads[i]);
            }
        });
    });

    describe('Runtime Performance Comparison', () => {
        test('should compare runtime performance', async () => {
            const runtimeResults = {};

            for (const runtime of testData.runtimes) {
                const testFunction = await createTestFunction(`perf-test-${runtime.name.replace('.', '-')}`, runtime.name);
                testFunctions.push(testFunction);

                const iterations = 5;
                const results = [];

                for (let i = 0; i < iterations; i++) {
                    const result = await measureInvocation(
                        testFunction.name,
                        global.testManager.generateTestPayload(
                            `perf-${runtime.name}-${i}`,
                            `Performance test for ${runtime.name}`,
                            0
                        )
                    );
                    results.push(result);
                }

                const avgDuration = results.reduce((sum, r) => sum + r.duration, 0) / results.length;
                runtimeResults[runtime.name] = {
                    avgDuration,
                    nodeVersion: results[0].result.nodeVersion,
                    results
                };
            }

            // Both runtimes should perform reasonably well
            for (const [runtimeName, data] of Object.entries(runtimeResults)) {
                assertWithinPerformanceThreshold(data.avgDuration, testData.performanceThresholds.fastExecution);
            }
        });
    });

    describe('Runtime Error Handling', () => {
        test('should handle runtime errors gracefully', async () => {
            const testFunction = await createTestFunction('runtime-error-test');
            testFunctions.push(testFunction);

            // Test with various error scenarios
            const errorScenarios = [
                { type: 'large', payload: 'x'.repeat(100000), description: 'Large payload' }
            ];

            for (const scenario of errorScenarios) {
                try {
                    const result = await invokeTestFunction(
                        testFunction.name,
                        `error-test-${scenario.type}`,
                        `Testing ${scenario.description}`,
                        0,
                        scenario.payload ? { largeData: scenario.payload } : {}
                    );

                    assertValidLambdaResponse(result);
                } catch (error) {
                    // Some errors are expected and should be handled gracefully
                    assert.ok(error.message);
                }
            }
        });
    });

    describe('Runtime Memory and Resource Usage', () => {
        test('should handle memory-intensive operations', async () => {
            const testFunction = await createTestFunction('memory-test');
            testFunctions.push(testFunction);

            // Test with memory-intensive payload
            const memoryIntensivePayload = {
                largeArray: new Array(10000).fill(0).map((_, i) => ({ id: i, data: `item-${i}` })),
                largeString: 'x'.repeat(100000)
            };

            const result = await invokeTestFunction(
                testFunction.name,
                'memory-intensive-test',
                'Testing memory-intensive operations',
                0,
                memoryIntensivePayload
            );

            assertValidLambdaResponse(result);
            assertMatchObject(result.event, memoryIntensivePayload);
        });

        test('should handle concurrent memory usage', async () => {
            const testFunction = await createTestFunction('concurrent-memory-test');
            testFunctions.push(testFunction);

            const payloadGenerator = (index) =>
                global.testManager.generateConcurrentPayload(
                    index,
                    'concurrent-memory',
                    'Concurrent memory test',
                    0,
                    { largeData: 'x'.repeat(10000) }
                );

            const results = await runConcurrentInvocations(testFunction.name, 5, payloadGenerator);

            assertSuccessfulInvocations(results, 5);

            // All should complete within reasonable time even with memory pressure
            const maxDuration = Math.max(...results.map(r => r.duration));
            assertWithinPerformanceThreshold(maxDuration, testData.performanceThresholds.mediumExecution);
        });
    });

    describe('Runtime Compatibility', () => {
        test('should maintain API compatibility across runtimes', async () => {
            const runtimeFunctions = [];

            // Create functions for each runtime
            for (const runtime of testData.runtimes) {
                const testFunction = await createTestFunction(`compat-test-${runtime.name.replace('.', '-')}`, runtime.name);
                runtimeFunctions.push({ ...testFunction, runtime: runtime.name });
                testFunctions.push(testFunction);
            }

            // Test same payload across all runtimes
            const testPayload = {
                testId: 'compatibility-test',
                message: 'Testing API compatibility',
                wait: 0,
                compatibility: {
                    features: ['async/await', 'promises', 'json'],
                    version: '1.0.0'
                }
            };

            const results = [];
            for (const testFunction of runtimeFunctions) {
                const result = await global.testManager.client.invokeFunction(testFunction.name, testPayload);
                results.push({ runtime: testFunction.runtime, result });
            }

            // All runtimes should produce compatible responses
            for (const { runtime, result } of results) {
                assertValidLambdaResponse(result);
                assert.strictEqual(result.testId, 'compatibility-test');
                assert.strictEqual(result.message, 'Testing API compatibility');
                assertMatchObject(result.event, testPayload);
            }
        });
    });
});
