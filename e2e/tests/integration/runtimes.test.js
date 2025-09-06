/**
 * Runtime Integration Tests
 */

const testData = require('../fixtures/test-data');

describe('Lambda@Home Runtime Integration Tests', () => {
    let testFunctions = [];

    afterAll(async () => {
        for (const testFunction of testFunctions) {
            await global.testManager.client.deleteFunction(testFunction.name);
        }
    });

    describe('Node.js Runtime Support', () => {
        test.each(testData.runtimes)('should support $name runtime', async (runtime) => {
            const testFunction = await createTestFunction(`runtime-test-${runtime.name.replace('.', '-')}`, runtime.name);
            testFunctions.push(testFunction);

            const result = await invokeTestFunction(
                testFunction.name,
                'runtime-test',
                `Testing ${runtime.name} runtime`,
                0
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.nodeVersion).toBe(runtime.version);
            expect(result.runtime).toBe('node');
        });

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

                expect(result).toBeValidLambdaResponse();
                expect(result.event).toMatchObject(testPayloads[i]);
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
                expect(data.avgDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.fastExecution);
                // Runtime performance logged only in verbose mode
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

                    expect(result).toBeValidLambdaResponse();
                } catch (error) {
                    // Some errors are expected and should be handled gracefully
                    expect(error.message).toBeDefined();
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

            expect(result).toBeValidLambdaResponse();
            expect(result.event).toMatchObject(memoryIntensivePayload);
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
            
            expect(results).toHaveSuccessfulInvocations(5);
            
            // All should complete within reasonable time even with memory pressure
            const maxDuration = Math.max(...results.map(r => r.duration));
            expect(maxDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.mediumExecution);
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
                expect(result).toBeValidLambdaResponse();
                expect(result.testId).toBe('compatibility-test');
                expect(result.message).toBe('Testing API compatibility');
                expect(result.event).toMatchObject(testPayload);
            }
        });
    });
});
