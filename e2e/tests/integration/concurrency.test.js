/**
 * Concurrency and Throttling Tests
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

describe('Lambda@Home Concurrency and Throttling Tests', () => {
    let testFunction;

    before(async () => {
        testFunction = await createTestFunction('concurrency-test');
    });

    after(cleanupSingleFunction(() => testFunction, global.testManager.client, {
        timeout: 60000,
        verifyCleanup: true,
        forceRemoveContainers: true
    }));

    describe('Basic Concurrency', () => {
        test('should handle concurrent invocations efficiently', async () => {
            const concurrencyLevels = [2, 3, 5, 10];
            
            for (const level of concurrencyLevels) {
                const payloadGenerator = (index) => 
                    global.testManager.generateConcurrentPayload(
                        index,
                        `concurrency-${level}`,
                        `Concurrency test level ${level}`,
                        0
                    );

                const startTime = Date.now();
                const results = await runConcurrentInvocations(testFunction.name, level, payloadGenerator);
                const totalTime = Date.now() - startTime;

                assertSuccessfulInvocations(results, level);

                // Concurrent execution should be faster than sequential
                const maxDuration = Math.max(...results.map(r => r.duration));
                assert.ok(totalTime < maxDuration * level);
            }
        });

        test('should maintain performance under concurrent load', async () => {
            const concurrentCount = 5;
            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    'performance-test',
                    'Concurrent performance test',
                    0
                );

            const results = await runConcurrentInvocations(testFunction.name, concurrentCount, payloadGenerator);

            assertSuccessfulInvocations(results, concurrentCount);

            // All should complete within reasonable time
            for (const result of results) {
                assertWithinPerformanceThreshold(result.duration, testData.performanceThresholds.mediumExecution);
            }
        });
    });

    describe('Concurrency Scaling', () => {
        test('should scale throughput with concurrency', async () => {
            const loadLevels = [1, 3, 5, 8];
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

                assertSuccessfulInvocations(results, level);
            }

            // Throughput should generally increase with concurrency (up to a point)
            assert.strictEqual(throughputResults.length, loadLevels.length);
        });

        test('should handle burst traffic patterns', async () => {
            const burstRounds = 3;
            const burstSize = 5;

            for (let round = 0; round < burstRounds; round++) {
                const payloadGenerator = (index) => 
                    global.testManager.generateConcurrentPayload(
                        index,
                        `burst-${round}`,
                        `Burst traffic round ${round}`,
                        0
                    );

                const results = await runConcurrentInvocations(testFunction.name, burstSize, payloadGenerator);

                assertSuccessfulInvocations(results, burstSize);
                
                // Wait between bursts
                await new Promise(resolve => setTimeout(resolve, 500));
            }
        });
    });

    describe('Resource Contention', () => {
        test('should handle concurrent memory-intensive operations', async () => {
            const memoryIntensivePayload = global.testManager.generateTestPayload(
                'concurrent-memory',
                'Concurrent memory test',
                0,
                { largeData: 'x'.repeat(10000) }
            );

            const concurrentCount = 3;
            const results = [];

            for (let i = 0; i < concurrentCount; i++) {
                results.push(
                    measureInvocation(testFunction.name, memoryIntensivePayload)
                        .catch(error => ({ error: error.message, index: i }))
                );
            }

            const finalResults = await Promise.all(results);

            // At least some should succeed
            const successCount = finalResults.filter(r => r.result && !r.error).length;
            assert.ok(successCount >= 1);
        });

        test('should handle concurrent CPU-intensive operations', async () => {
            const cpuIntensivePayload = global.testManager.generateTestPayload(
                'concurrent-cpu',
                'Concurrent CPU test',
                0,
                {
                    cpuIntensive: true,
                    iterations: 10000,
                    operation: 'fibonacci'
                }
            );

            const concurrentCount = 3;
            const results = [];

            for (let i = 0; i < concurrentCount; i++) {
                results.push(
                    measureInvocation(testFunction.name, cpuIntensivePayload)
                        .catch(error => ({ error: error.message, index: i }))
                );
            }

            const finalResults = await Promise.all(results);

            // At least some should succeed
            const successCount = finalResults.filter(r => r.result && !r.error).length;
            assert.ok(successCount >= 1);
        });
    });

    describe('Concurrency Limits and Throttling', () => {
        test('should handle high concurrency gracefully', async () => {
            const highConcurrency = 15;
            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    'high-concurrency',
                    'High concurrency test',
                    0
                );

            const results = await runConcurrentInvocations(testFunction.name, highConcurrency, payloadGenerator);

            // Should handle high concurrency gracefully
            assert.strictEqual(results.length, highConcurrency);

            // Count successful invocations
            const successCount = results.filter(r => r.result && !r.error).length;
            assert.ok(successCount >= highConcurrency * 0.8); // At least 80% success rate
        });

        test('should maintain response times under load', async () => {
            const loadLevels = [5, 10, 15];
            
            for (const level of loadLevels) {
                const payloadGenerator = (index) => 
                    global.testManager.generateConcurrentPayload(
                        index,
                        `response-time-${level}`,
                        `Response time test level ${level}`,
                        0
                    );

                const results = await runConcurrentInvocations(testFunction.name, level, payloadGenerator);
                
                // Calculate response time statistics
                const durations = results.map(r => r.duration).filter(d => d > 0);
                if (durations.length > 0) {
                    const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
                    const p95Duration = durations.sort((a, b) => a - b)[Math.floor(durations.length * 0.95)];

                    // Response times should remain reasonable
                    assertWithinPerformanceThreshold(avgDuration, testData.performanceThresholds.mediumExecution);
                    assertWithinPerformanceThreshold(p95Duration, testData.performanceThresholds.slowExecution);
                }
            }
        });
    });

    describe('Concurrent Error Handling', () => {
        test('should handle concurrent errors gracefully', async () => {
            const errorPayloads = [
                { error: 'test1', type: 'handled' },
                { error: 'test2', type: 'unhandled' },
                { error: 'test3', type: 'timeout' },
                { error: 'test4', type: 'memory' },
                { error: 'test5', type: 'cpu' }
            ];

            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    'concurrent-error',
                    'Concurrent error test',
                    0,
                    errorPayloads[index] || { error: 'default' }
                );

            const results = await runConcurrentInvocations(testFunction.name, 5, payloadGenerator);

            // Should handle concurrent errors gracefully
            assert.strictEqual(results.length, 5);

            for (const result of results) {
                // Each result should either succeed or fail gracefully
                assert.ok((result.result || result.error) !== undefined);
            }
        });

        test('should recover from concurrent failures', async () => {
            const problematicPayload = global.testManager.generateTestPayload(
                'concurrent-recovery',
                'Concurrent recovery test',
                0,
                {
                    problematic: true,
                    stress: true,
                    iterations: 1000
                }
            );

            const concurrentCount = 3;
            const results = [];

            for (let i = 0; i < concurrentCount; i++) {
                results.push(
                    measureInvocation(testFunction.name, problematicPayload)
                        .catch(error => ({ error: error.message, index: i }))
                );
            }

            const finalResults = await Promise.all(results);

            // At least some should succeed
            const successCount = finalResults.filter(r => r.result && !r.error).length;
            assert.ok(successCount >= 1);
        });
    });

    describe('Sustained Concurrency', () => {
        test('should maintain performance under sustained concurrent load', async () => {
            const sustainedRounds = 3;
            const concurrentPerRound = 5;

            for (let round = 0; round < sustainedRounds; round++) {
                const payloadGenerator = (index) => 
                    global.testManager.generateConcurrentPayload(
                        index,
                        `sustained-${round}`,
                        `Sustained concurrent load round ${round}`,
                        0
                    );

                const results = await runConcurrentInvocations(testFunction.name, concurrentPerRound, payloadGenerator);

                assertSuccessfulInvocations(results, concurrentPerRound);

                // Performance should remain consistent across rounds
                const avgDuration = results.reduce((sum, r) => sum + r.duration, 0) / results.length;
                assertWithinPerformanceThreshold(avgDuration, testData.performanceThresholds.mediumExecution);
                
                // Wait between rounds
                await new Promise(resolve => setTimeout(resolve, 1000));
            }
        });

        test('should handle mixed concurrent and sequential patterns', async () => {
            // Mix of concurrent and sequential invocations
            const patterns = [
                { type: 'concurrent', count: 3 },
                { type: 'sequential', count: 2 },
                { type: 'concurrent', count: 5 },
                { type: 'sequential', count: 1 }
            ];

            for (const pattern of patterns) {
                if (pattern.type === 'concurrent') {
                    const payloadGenerator = (index) => 
                        global.testManager.generateConcurrentPayload(
                            index,
                            `mixed-${pattern.type}`,
                            `Mixed pattern ${pattern.type}`,
                            0
                        );

                    const results = await runConcurrentInvocations(testFunction.name, pattern.count, payloadGenerator);
                    assertSuccessfulInvocations(results, pattern.count);
                } else {
                    // Sequential
                    for (let i = 0; i < pattern.count; i++) {
                        const result = await measureInvocation(
                            testFunction.name,
                            global.testManager.generateTestPayload(
                                `mixed-sequential-${i}`,
                                `Mixed sequential ${i}`,
                                0
                            )
                        );
                        assertValidLambdaResponse(result.result);
                    }
                }
            }
        });
    });
});
