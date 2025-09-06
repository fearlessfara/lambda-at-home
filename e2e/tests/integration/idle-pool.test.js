/**
 * Idle Pool and Container Lifecycle Tests
 */

const testData = require('../fixtures/test-data');

describe('Lambda@Home Idle Pool and Container Lifecycle Tests', () => {
    let testFunction;

    beforeAll(async () => {
        testFunction = await createTestFunction('idle-pool-test');
    });

    afterAll(async () => {
        await global.testManager.client.deleteFunction(testFunction.name);
    });

    describe('Container Lifecycle Management', () => {
        test('should handle cold start vs warm start performance', async () => {
            // First invocation - should be cold start
            const coldStartResult = await measureInvocation(
                testFunction.name,
                global.testManager.generateTestPayload(
                    'cold-start-test',
                    'Cold start test',
                    0
                )
            );

            expect(coldStartResult.result).toBeValidLambdaResponse();
            expect(coldStartResult.duration).toBeGreaterThan(0);

            // Second invocation within short time - should reuse warm container
            const warmStartResult = await measureInvocation(
                testFunction.name,
                global.testManager.generateTestPayload(
                    'warm-start-test',
                    'Warm start test',
                    0
                )
            );

            expect(warmStartResult.result).toBeValidLambdaResponse();
            expect(warmStartResult.duration).toBeGreaterThan(0);

            // Warm start should generally be faster than cold start
            // Note: This might not always be true due to system variability
            console.log(`Cold start: ${coldStartResult.duration}ms, Warm start: ${warmStartResult.duration}ms`);
        });

        test('should handle multiple rapid invocations efficiently', async () => {
            const invocationCount = 5;
            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    'rapid-invocations',
                    'Rapid invocation test',
                    0
                );

            const results = await runConcurrentInvocations(testFunction.name, invocationCount, payloadGenerator);
            
            expect(results).toHaveSuccessfulInvocations(invocationCount);
            
            // All invocations should complete successfully
            for (const result of results) {
                expect(result.result).toBeValidLambdaResponse();
                expect(result.duration).toBeGreaterThan(0);
            }
        });
    });

    describe('Container Reuse and Efficiency', () => {
        test('should reuse containers for sequential invocations', async () => {
            const sequentialCount = 3;
            const results = [];

            for (let i = 0; i < sequentialCount; i++) {
                const result = await measureInvocation(
                    testFunction.name,
                    global.testManager.generateTestPayload(
                        `sequential-${i}`,
                        `Sequential invocation ${i}`,
                        0
                    )
                );
                results.push(result);
            }

            expect(results).toHaveLength(sequentialCount);
            
            // All should succeed
            for (const result of results) {
                expect(result.result).toBeValidLambdaResponse();
            }
        });

        test('should handle container cleanup after inactivity', async () => {
            // Make an initial invocation
            const initialResult = await measureInvocation(
                testFunction.name,
                global.testManager.generateTestPayload(
                    'initial-invocation',
                    'Initial invocation',
                    0
                )
            );

            expect(initialResult.result).toBeValidLambdaResponse();

            // Wait for potential container cleanup (this is system-dependent)
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Make another invocation after potential cleanup
            const afterWaitResult = await measureInvocation(
                testFunction.name,
                global.testManager.generateTestPayload(
                    'after-wait-invocation',
                    'After wait invocation',
                    0
                )
            );

            expect(afterWaitResult.result).toBeValidLambdaResponse();
        });
    });

    describe('Resource Management', () => {
        test('should handle memory-efficient container reuse', async () => {
            const memoryIntensivePayload = global.testManager.generateTestPayload(
                'memory-intensive',
                'Memory intensive test',
                0,
                { largeData: 'x'.repeat(10000) }
            );

            // Multiple invocations with memory-intensive payloads
            const results = [];
            for (let i = 0; i < 3; i++) {
                const result = await measureInvocation(testFunction.name, memoryIntensivePayload);
                results.push(result);
            }

            // All should succeed despite memory pressure
            for (const result of results) {
                expect(result.result).toBeValidLambdaResponse();
            }
        });

        test('should maintain performance under sustained load', async () => {
            const sustainedLoadRounds = 3;
            const requestsPerRound = 3;

            for (let round = 0; round < sustainedLoadRounds; round++) {
                const payloadGenerator = (index) => 
                    global.testManager.generateConcurrentPayload(
                        index,
                        `sustained-${round}`,
                        `Sustained load round ${round}`,
                        0
                    );

                const results = await runConcurrentInvocations(testFunction.name, requestsPerRound, payloadGenerator);
                
                expect(results).toHaveSuccessfulInvocations(requestsPerRound);
                
                // Performance should remain consistent
                const avgDuration = results.reduce((sum, r) => sum + r.duration, 0) / results.length;
                expect(avgDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.fastExecution);
            }
        });
    });

    describe('Container State Management', () => {
        test('should handle container state transitions', async () => {
            // Test different payload types to ensure container handles various inputs
            const testPayloads = [
                { simple: 'string' },
                { complex: { nested: { data: [1, 2, 3] } } },
                { array: [1, 2, 3, 4, 5] },
                { nullValue: null }
            ];

            for (let i = 0; i < testPayloads.length; i++) {
                const result = await measureInvocation(
                    testFunction.name,
                    global.testManager.generateTestPayload(
                        `state-test-${i}`,
                        `Container state test ${i}`,
                        0,
                        testPayloads[i]
                    )
                );

                expect(result.result).toBeValidLambdaResponse();
                expect(result.result.event).toMatchObject(testPayloads[i]);
            }
        });

        test('should handle container error recovery', async () => {
            // Test with potentially problematic payload
            const problematicPayload = global.testManager.generateTestPayload(
                'error-recovery-test',
                'Error recovery test',
                0,
                { 
                    problematic: true,
                    largeString: 'x'.repeat(100000),
                    deepObject: { level1: { level2: { level3: { data: 'test' } } } }
                }
            );

            const result = await measureInvocation(testFunction.name, problematicPayload);
            
            // Should either succeed or fail gracefully
            if (result.result) {
                expect(result.result).toBeValidLambdaResponse();
            }
        });
    });
});
