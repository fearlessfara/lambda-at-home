/**
 * Service Integration Tests
 */

const testData = require('../fixtures/test-data');

describe('Lambda@Home Service Integration Tests', () => {
    let testFunction;

    beforeAll(async () => {
        testFunction = await createTestFunction('service-test');
    });

    afterAll(async () => {
        await global.testManager.client.deleteFunction(testFunction.name);
    });

    describe('Health and Metrics Endpoints', () => {
        test('should have healthy server', async () => {
            const health = await global.testManager.client.healthCheck();
            expect(health.healthy).toBe(true);
        });

        test('should provide metrics endpoint', async () => {
            const metrics = await global.testManager.client.getMetrics();
            expect(metrics).toContain('lambda_cold_starts_total');
            expect(metrics).toContain('lambda_duration_ms');
        });
    });

    describe('Function Management', () => {
        test('should create function successfully', () => {
            expect(testFunction.name).toBeDefined();
            expect(testFunction.data.function_name).toBe(testFunction.name);
            expect(testFunction.data.state).toBe('Active');
        });

        test('should list functions', async () => {
            const functions = await global.testManager.client.listFunctions();
            expect(functions.functions).toBeDefined();
            expect(Array.isArray(functions.functions)).toBe(true);
            
            const ourFunction = functions.functions.find(f => f.function_name === testFunction.name);
            expect(ourFunction).toBeDefined();
        });

        test('should get function details', async () => {
            const functionData = await global.testManager.client.getFunction(testFunction.name);
            expect(functionData.function_name).toBe(testFunction.name);
            expect(functionData.runtime).toBe('nodejs22.x');
            expect(functionData.handler).toBe('index.handler');
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

            expect(result).toBeValidLambdaResponse();
            expect(result.testId).toBe('service-test');
            expect(result.message).toBe('Hello from service test');
            expect(result.waitMs).toBe(100);
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

                expect(result).toBeValidLambdaResponse();
                expect(result.waitMs).toBe(scenario.wait);
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
            
            expect(results).toHaveSuccessfulInvocations(3);
            
            // Check that all invocations completed within reasonable time
            const maxDuration = Math.max(...results.map(r => r.duration));
            expect(maxDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.concurrentExecution);
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
            
            expect(results).toHaveLength(5);
            
            for (const result of results) {
                expect(result.result).toBeValidLambdaResponse();
                expect(result.duration).toBeWithinPerformanceThreshold(testData.performanceThresholds.mediumExecution);
            }
        });
    });

    describe('Error Handling', () => {
        test('should handle invalid function name', async () => {
            await expect(
                global.testManager.client.invokeFunction('non-existent-function', { test: 'data' })
            ).rejects.toThrow();
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

            expect(result).toBeValidLambdaResponse();
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

            expect(avgDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.fastExecution);
            expect(maxDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.mediumExecution);
            expect(minDuration).toBeGreaterThanOrEqual(0);

            // Performance stats logged only in verbose mode
        });
    });
});
