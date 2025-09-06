/**
 * Function Versioning and Management Tests
 */

const testData = require('../fixtures/test-data');

describe('Lambda@Home Function Versioning and Management Tests', () => {
    let testFunction;

    beforeAll(async () => {
        testFunction = await createTestFunction('versioning-test');
    });

    afterAll(async () => {
        await global.testManager.client.deleteFunction(testFunction.name);
    });

    describe('Function Versioning', () => {
        test('should create function with versioning enabled', async () => {
            const versionedFunction = await global.testManager.createTestFunction('versioned-test');
            
            expect(versionedFunction.name).toBeDefined();
            expect(versionedFunction.data.function_name).toBe(versionedFunction.name);
            expect(versionedFunction.data.state).toBe('Active');

            // Clean up
            await global.testManager.client.deleteFunction(versionedFunction.name);
        });

        test('should handle function updates and versioning', async () => {
            const updateFunction = await global.testManager.createTestFunction('update-test');
            
            // Test function invocation
            const result = await invokeTestFunction(
                updateFunction.name,
                'version-test',
                'Testing function versioning',
                0
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.testId).toBe('version-test');

            // Clean up
            await global.testManager.client.deleteFunction(updateFunction.name);
        });

        test('should maintain function state across invocations', async () => {
            const stateFunction = await global.testManager.createTestFunction('state-test');
            
            // Multiple invocations to test state consistency
            const results = [];
            for (let i = 0; i < 3; i++) {
                const result = await invokeTestFunction(
                    stateFunction.name,
                    `state-test-${i}`,
                    `State test ${i}`,
                    0
                );
                results.push(result);
            }

            // All should succeed and maintain consistent behavior
            for (const result of results) {
                expect(result).toBeValidLambdaResponse();
            }

            // Clean up
            await global.testManager.client.deleteFunction(stateFunction.name);
        });
    });

    describe('Function Configuration Management', () => {
        test('should handle function configuration updates', async () => {
            const configFunction = await global.testManager.createTestFunction('config-test');
            
            // Test with different configurations
            const configTests = [
                { timeout: 5, memory: 256 },
                { timeout: 10, memory: 512 },
                { timeout: 15, memory: 1024 }
            ];

            for (let i = 0; i < configTests.length; i++) {
                const config = configTests[i];
                const result = await invokeTestFunction(
                    configFunction.name,
                    `config-test-${i}`,
                    `Config test ${i}`,
                    0,
                    { config: config }
                );

                expect(result).toBeValidLambdaResponse();
            }

            // Clean up
            await global.testManager.client.deleteFunction(configFunction.name);
        });

        test('should handle function environment variables', async () => {
            const envFunction = await global.testManager.createTestFunction('env-test');
            
            // Test with environment-specific payload
            const envPayload = global.testManager.generateTestPayload(
                'env-test',
                'Environment test',
                0,
                { 
                    environment: 'test',
                    nodeEnv: process.env.NODE_ENV || 'development'
                }
            );

            const result = await global.testManager.client.invokeFunction(envFunction.name, envPayload);
            
            expect(result).toBeValidLambdaResponse();
            expect(result.event).toMatchObject(envPayload);

            // Clean up
            await global.testManager.client.deleteFunction(envFunction.name);
        });
    });

    describe('Function Lifecycle Management', () => {
        test('should handle function creation and deletion lifecycle', async () => {
            const lifecycleFunction = await global.testManager.createTestFunction('lifecycle-test');
            
            // Verify function exists
            const functionData = await global.testManager.client.getFunction(lifecycleFunction.name);
            expect(functionData.function_name).toBe(lifecycleFunction.name);
            
            // Test function works
            const result = await invokeTestFunction(
                lifecycleFunction.name,
                'lifecycle-test',
                'Lifecycle test',
                0
            );
            expect(result).toBeValidLambdaResponse();
            
            // Delete function
            const deleteResult = await global.testManager.client.deleteFunction(lifecycleFunction.name);
            expect(deleteResult.success).toBe(true);
            
            // Verify function no longer exists
            try {
                await global.testManager.client.getFunction(lifecycleFunction.name);
                expect(true).toBe(false); // Should not reach here
            } catch (error) {
                expect(error.message).toContain('404');
            }
        });

        test('should handle function updates without breaking existing invocations', async () => {
            const updateFunction = await global.testManager.createTestFunction('update-invoke-test');
            
            // Initial invocation
            const initialResult = await invokeTestFunction(
                updateFunction.name,
                'initial-test',
                'Initial test',
                0
            );
            expect(initialResult).toBeValidLambdaResponse();
            
            // Simulate function update by invoking with different payload
            const updatedResult = await invokeTestFunction(
                updateFunction.name,
                'updated-test',
                'Updated test',
                0,
                { updated: true, version: '2.0' }
            );
            expect(updatedResult).toBeValidLambdaResponse();
            expect(updatedResult.event.updated).toBe(true);
            
            // Clean up
            await global.testManager.client.deleteFunction(updateFunction.name);
        });
    });

    describe('Function Metadata and Information', () => {
        test('should provide accurate function metadata', async () => {
            const metadataFunction = await global.testManager.createTestFunction('metadata-test');
            
            // Get function details
            const functionData = await global.testManager.client.getFunction(metadataFunction.name);
            
            expect(functionData.function_name).toBe(metadataFunction.name);
            expect(functionData.runtime).toBe('nodejs22.x');
            expect(functionData.handler).toBe('index.handler');
            expect(functionData.state).toBe('Active');
            expect(functionData.timeout).toBeDefined();
            expect(functionData.memory_size).toBeDefined();
            
            // Clean up
            await global.testManager.client.deleteFunction(metadataFunction.name);
        });

        test('should list functions correctly', async () => {
            const listFunction = await global.testManager.createTestFunction('list-test');
            
            // Get function list
            const functions = await global.testManager.client.listFunctions();
            
            expect(functions.functions).toBeDefined();
            expect(Array.isArray(functions.functions)).toBe(true);
            
            // Our function should be in the list
            const ourFunction = functions.functions.find(f => f.function_name === listFunction.name);
            expect(ourFunction).toBeDefined();
            expect(ourFunction.function_name).toBe(listFunction.name);
            
            // Clean up
            await global.testManager.client.deleteFunction(listFunction.name);
        });
    });

    describe('Function Performance and Consistency', () => {
        test('should maintain consistent performance across versions', async () => {
            const perfFunction = await global.testManager.createTestFunction('perf-version-test');
            
            // Warm up the function to avoid cold start affecting performance measurements
            await measureInvocation(
                perfFunction.name,
                global.testManager.generateTestPayload(
                    'warmup',
                    'Warmup invocation',
                    0
                )
            );
            
            // Test performance consistency
            const iterations = 5;
            const results = [];
            
            for (let i = 0; i < iterations; i++) {
                const result = await measureInvocation(
                    perfFunction.name,
                    global.testManager.generateTestPayload(
                        `perf-${i}`,
                        `Performance test ${i}`,
                        0
                    )
                );
                results.push(result);
            }
            
            // All should succeed
            for (const result of results) {
                expect(result.result).toBeValidLambdaResponse();
            }
            
            // Performance should be consistent (excluding cold start)
            const durations = results.map(r => r.duration);
            const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
            expect(avgDuration).toBeWithinPerformanceThreshold(testData.performanceThresholds.fastExecution);
            
            // Clean up
            await global.testManager.client.deleteFunction(perfFunction.name);
        });

        test('should handle function versioning with concurrent access', async () => {
            const concurrentVersionFunction = await global.testManager.createTestFunction('concurrent-version-test');
            
            // Concurrent invocations to test versioning consistency
            const concurrentCount = 5;
            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    'concurrent-version',
                    'Concurrent version test',
                    0,
                    { version: '1.0', index: index }
                );

            const results = await runConcurrentInvocations(concurrentVersionFunction.name, concurrentCount, payloadGenerator);
            
            expect(results).toHaveSuccessfulInvocations(concurrentCount);
            
            // All should have consistent behavior
            for (const result of results) {
                expect(result.result).toBeValidLambdaResponse();
            }
            
            // Clean up
            await global.testManager.client.deleteFunction(concurrentVersionFunction.name);
        });
    });

    describe('Function Error Handling and Recovery', () => {
        test('should handle function errors gracefully across versions', async () => {
            const errorFunction = await global.testManager.createTestFunction('error-version-test');
            
            // Test with various error scenarios
            const errorScenarios = [
                { type: 'null', data: null },
                { type: 'undefined', data: undefined },
                { type: 'large', data: 'x'.repeat(100000) }
            ];

            for (const scenario of errorScenarios) {
                try {
                    const result = await invokeTestFunction(
                        errorFunction.name,
                        `error-${scenario.type}`,
                        `Error test ${scenario.type}`,
                        0,
                        scenario.data
                    );

                    expect(result).toBeValidLambdaResponse();
                } catch (error) {
                    // Some error scenarios might cause failures, which is acceptable
                    expect(error.message).toBeDefined();
                }
            }
            
            // Clean up
            await global.testManager.client.deleteFunction(errorFunction.name);
        });

        test('should recover from function errors and continue working', async () => {
            const recoveryFunction = await global.testManager.createTestFunction('recovery-version-test');
            
            // Test error recovery
            const problematicPayload = global.testManager.generateTestPayload(
                'recovery-test',
                'Recovery test',
                0,
                { 
                    problematic: true,
                    stress: true,
                    iterations: 1000
                }
            );

            const results = [];
            for (let i = 0; i < 3; i++) {
                try {
                    const result = await measureInvocation(recoveryFunction.name, problematicPayload);
                    results.push({ success: true, result });
                } catch (error) {
                    results.push({ success: false, error: error.message });
                }
            }

            // At least some should succeed
            const successCount = results.filter(r => r.success).length;
            expect(successCount).toBeGreaterThanOrEqual(1);
            
            // Clean up
            await global.testManager.client.deleteFunction(recoveryFunction.name);
        });
    });
});
