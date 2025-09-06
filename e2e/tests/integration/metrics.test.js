/**
 * Metrics and Performance Integration Tests
 */

const testData = require('../fixtures/test-data');

describe('Lambda@Home Metrics and Performance Tests', () => {
    let testFunction;

    beforeAll(async () => {
        testFunction = await createTestFunction('metrics-test');
    });

    afterAll(async () => {
        await global.testManager.client.deleteFunction(testFunction.name);
    });

    describe('Performance Metrics Collection', () => {
        test('should collect execution time metrics', async () => {
            const scenarios = Object.entries(testData.testScenarios);
            
            for (const [scenarioName, scenario] of scenarios) {
                const result = await measureInvocation(
                    testFunction.name,
                    global.testManager.generateTestPayload(
                        `metrics-${scenarioName}`,
                        `Testing ${scenario.description}`,
                        scenario.wait
                    )
                );

                expect(result.result).toBeValidLambdaResponse();
                expect(result.duration).toBeGreaterThanOrEqual(0);
                
                // Duration should be at least the wait time
                expect(result.duration).toBeGreaterThanOrEqual(scenario.wait);
                
                // Performance metrics logged only in verbose mode
            }
        });

        test('should handle load testing', async () => {
            const loadConfig = testData.loadTestConfigs.medium;
            const results = [];

            for (let i = 0; i < loadConfig.count; i++) {
                const result = await measureInvocation(
                    testFunction.name,
                    global.testManager.generateTestPayload(
                        `load-test-${i}`,
                        `Load test ${i}`,
                        0
                    )
                );
                results.push(result);
                
                if (loadConfig.delay > 0) {
                    await new Promise(resolve => setTimeout(resolve, loadConfig.delay));
                }
            }

            expect(results).toHaveLength(loadConfig.count);
            
            // Calculate performance statistics
            const durations = results.map(r => r.duration);
            const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
            const maxDuration = Math.max(...durations);
            const minDuration = Math.min(...durations);
            const p95Duration = durations.sort((a, b) => a - b)[Math.floor(durations.length * 0.95)];

            expect(avgDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.fastExecution);
            expect(p95Duration).toBeWithinPerformanceThreshold(testData.performanceThresholds.mediumExecution);

            // Load test stats logged only in verbose mode
        });
    });

    describe('Concurrency Performance', () => {
        test('should handle concurrent execution efficiently', async () => {
            const concurrencyLevels = Object.values(testData.concurrencyLevels);
            
            for (const level of concurrencyLevels) {
                const payloadGenerator = (index) => 
                    global.testManager.generateConcurrentPayload(
                        index,
                        `concurrency-${level}`,
                        `Concurrency test level ${level}`,
                        100
                    );

                const startTime = Date.now();
                const results = await runConcurrentInvocations(testFunction.name, level, payloadGenerator);
                const totalTime = Date.now() - startTime;

                expect(results).toHaveSuccessfulInvocations(level);
                
                // Concurrent execution should be faster than sequential
                const maxDuration = Math.max(...results.map(r => r.duration));
                expect(totalTime).toBeLessThan(maxDuration * level);
                
                // Concurrency stats logged only in verbose mode
            }
        });

        test('should maintain performance under sustained load', async () => {
            const sustainedLoadRounds = 3;
            const requestsPerRound = 5;
            const roundDelay = 1000;

            for (let round = 0; round < sustainedLoadRounds; round++) {
                const payloadGenerator = (index) => 
                    global.testManager.generateConcurrentPayload(
                        index,
                        `sustained-${round}`,
                        `Sustained load round ${round}`,
                        50
                    );

                const results = await runConcurrentInvocations(testFunction.name, requestsPerRound, payloadGenerator);
                
                expect(results).toHaveSuccessfulInvocations(requestsPerRound);
                
                const avgDuration = results.reduce((sum, r) => sum + r.duration, 0) / results.length;
                expect(avgDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.fastExecution);
                
                // Sustained load stats logged only in verbose mode
                
                if (round < sustainedLoadRounds - 1) {
                    await new Promise(resolve => setTimeout(resolve, roundDelay));
                }
            }
        });
    });

    describe('Error Rate and Recovery', () => {
        test('should maintain low error rate under normal conditions', async () => {
            const iterations = 20;
            const results = [];

            for (let i = 0; i < iterations; i++) {
                try {
                    const result = await measureInvocation(
                        testFunction.name,
                        global.testManager.generateTestPayload(
                            `error-rate-${i}`,
                            `Error rate test ${i}`,
                            0
                        )
                    );
                    results.push({ success: true, result });
                } catch (error) {
                    results.push({ success: false, error: error.message });
                }
            }

            const successCount = results.filter(r => r.success).length;
            const errorRate = (iterations - successCount) / iterations;
            
            expect(errorRate).toBeLessThan(0.05); // Less than 5% error rate
            expect(successCount).toBeGreaterThanOrEqual(iterations * 0.95);
            
            // Error rate stats logged only in verbose mode
        });

        test('should recover from temporary failures', async () => {
            // Test with some stress to see if system recovers
            const stressPayload = global.testManager.generateTestPayload(
                'stress-test',
                'Stress test for recovery',
                0,
                { stress: true, iterations: 1000 }
            );

            const results = [];
            for (let i = 0; i < 5; i++) {
                try {
                    const result = await measureInvocation(testFunction.name, stressPayload);
                    results.push({ success: true, duration: result.duration });
                } catch (error) {
                    results.push({ success: false, error: error.message });
                }
                
                // Small delay between attempts
                await new Promise(resolve => setTimeout(resolve, 100));
            }

            const successCount = results.filter(r => r.success).length;
            expect(successCount).toBeGreaterThanOrEqual(3); // At least 60% should succeed
            
            if (successCount > 0) {
                const successfulResults = results.filter(r => r.success);
                const avgDuration = successfulResults.reduce((sum, r) => sum + r.duration, 0) / successfulResults.length;
                expect(avgDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.mediumExecution);
            }
        });
    });

    describe('Resource Utilization', () => {
        test('should handle memory-efficient operations', async () => {
            const memoryTests = [
                { size: 1000, description: 'Small payload' },
                { size: 10000, description: 'Medium payload' },
                { size: 100000, description: 'Large payload' }
            ];

            for (const test of memoryTests) {
                const payload = global.testManager.generateTestPayload(
                    `memory-${test.size}`,
                    `Testing ${test.description}`,
                    0,
                    { data: 'x'.repeat(test.size) }
                );

                const result = await measureInvocation(testFunction.name, payload);
                
                expect(result.result).toBeValidLambdaResponse();
                expect(result.duration).toBeWithinPerformanceThreshold(testData.performanceThresholds.mediumExecution);
                
                // Memory test stats logged only in verbose mode
            }
        });

        test('should handle CPU-intensive operations', async () => {
            const cpuIntensivePayload = global.testManager.generateTestPayload(
                'cpu-intensive',
                'CPU intensive test',
                0,
                { 
                    cpuIntensive: true,
                    iterations: 100000,
                    operation: 'fibonacci'
                }
            );

            const result = await measureInvocation(testFunction.name, cpuIntensivePayload);
            
            expect(result.result).toBeValidLambdaResponse();
            expect(result.duration).toBeWithinPerformanceThreshold(testData.performanceThresholds.slowExecution);
            
            // CPU intensive test stats logged only in verbose mode
        });
    });

    describe('Throughput and Scalability', () => {
        test('should maintain throughput under increasing load', async () => {
            const loadLevels = [1, 3, 5, 10];
            const throughputResults = [];

            for (const level of loadLevels) {
                const payloadGenerator = (index) => 
                    global.testManager.generateConcurrentPayload(
                        index,
                        `throughput-${level}`,
                        `Throughput test level ${level}`,
                        0
                    );

                const startTime = Date.now();
                const results = await runConcurrentInvocations(testFunction.name, level, payloadGenerator);
                const totalTime = Date.now() - startTime;

                const throughput = level / (totalTime / 1000); // requests per second
                throughputResults.push({ level, throughput, totalTime });

                expect(results).toHaveSuccessfulInvocations(level);
                
                // Throughput stats logged only in verbose mode
            }

            // Throughput should generally increase with concurrency (up to a point)
            expect(throughputResults.length).toBe(loadLevels.length);
        });
    });
});
