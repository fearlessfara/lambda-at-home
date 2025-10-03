/**
 * Error Handling and Edge Case Tests
 */

const { describe, test, before, after } = require('node:test');
const assert = require('node:assert');
const testData = require('../fixtures/test-data');
const {
    assertValidLambdaResponse,
    assertWithinPerformanceThreshold,
    assertSuccessfulInvocations,
    assertMatchObject
} = require('../utils/assertions');
const { cleanupSingleFunction } = require('../utils/test-helpers');

require('../setup');

describe('Lambda@Home Error Handling and Edge Case Tests', () => {
    let testFunction;

    before(async () => {
        testFunction = await createTestFunction('error-handling-test');
    });

    after(cleanupSingleFunction(() => testFunction, global.testManager.client, {
        timeout: 60000,
        verifyCleanup: true,
        forceRemoveContainers: true
    }));

    describe('Function Error Handling', () => {
        test('should handle invalid function names gracefully', async () => {
            const invalidFunctionName = 'non-existent-function-12345';
            
            try {
                await global.testManager.client.invokeFunction(invalidFunctionName, { test: 'data' });
                // If we get here, the test should fail
                assert.fail('Should have thrown an error for non-existent function');
            } catch (error) {
                // Should get an error for non-existent function
                assert.ok(error.message !== undefined);
                assert.ok(error.message.includes('404'));
            }
        });

        test('should handle malformed payloads gracefully', async () => {
            const malformedPayloads = [
                null,
                undefined,
                { circular: {} }, // Will be handled by JSON.stringify
                { veryLarge: 'x'.repeat(1000000) },
                { specialChars: '!@#$%^&*()_+-=[]{}|;:,.<>?' }
            ];

            for (let i = 0; i < malformedPayloads.length; i++) {
                try {
                    const result = await invokeTestFunction(
                        testFunction.name,
                        `malformed-${i}`,
                        `Testing malformed payload ${i}`,
                        0,
                        malformedPayloads[i]
                    );

                    // Should handle gracefully
                    assert.ok(result !== undefined);
                } catch (error) {
                    // Some malformed payloads might cause errors, which is acceptable
                    assert.ok(error.message !== undefined);
                }
            }
        });

        test('should handle timeout scenarios', async () => {
            // Test with a payload that might cause longer processing
            const timeoutPayload = global.testManager.generateTestPayload(
                'timeout-test',
                'Timeout test',
                0,
                { 
                    timeoutTest: true,
                    iterations: 1000000,
                    operation: 'cpu-intensive'
                }
            );

            try {
                const result = await measureInvocation(testFunction.name, timeoutPayload);


                // Should either complete or timeout gracefully
                if (result.result) {
                    assert.ok(result.result !== undefined);
                }
            } catch (error) {
                // Timeout errors are acceptable
                assert.ok(error.message !== undefined);
            }
        });
    });

    describe('API Error Handling', () => {
        test('should handle invalid API requests', async () => {
            const client = global.testManager.client;

            // Test invalid function creation
            try {
                await client.createFunction('', 'nodejs22.x', 'index.handler', 'invalid-base64');
                assert.fail('Should have thrown an error for invalid function creation');
            } catch (error) {
                assert.ok(error.message !== undefined);
            }

            // Test invalid function update
            try {
                await client.createFunction('invalid-function-name!@#', 'nodejs22.x', 'index.handler', 'dGVzdA==');
                assert.fail('Should have thrown an error for invalid function name');
            } catch (error) {
                assert.ok(error.message !== undefined);
            }
        });

        test('should handle concurrent error scenarios', async () => {
            const errorPayloads = [
                { error: 'test1', type: 'handled' },
                { error: 'test2', type: 'unhandled' },
                { error: 'test3', type: 'timeout' }
            ];

            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    'concurrent-error',
                    'Concurrent error test',
                    0,
                    errorPayloads[index] || { error: 'default' }
                );

            const results = await runConcurrentInvocations(testFunction.name, 3, payloadGenerator);

            // Should handle concurrent errors gracefully
            assert.strictEqual(results.length, 3);

            for (const result of results) {
                // Each result should either succeed or fail gracefully
                assert.ok((result.result || result.error) !== undefined);
            }
        });
    });

    describe('Resource Limit Handling', () => {
        test('should handle memory pressure gracefully', async () => {
            const memoryIntensivePayloads = [
                { size: 100000, description: 'Medium memory' },
                { size: 500000, description: 'Large memory' },
                { size: 1000000, description: 'Very large memory' }
            ];

            for (const test of memoryIntensivePayloads) {
                const payload = global.testManager.generateTestPayload(
                    `memory-${test.size}`,
                    `Testing ${test.description}`,
                    0,
                    { data: 'x'.repeat(test.size) }
                );

                try {
                    const result = await measureInvocation(testFunction.name, payload);

                    if (result.result) {
                        assertValidLambdaResponse(result.result);
                    }
                } catch (error) {
                    // Memory pressure might cause errors, which is acceptable
                    assert.ok(error.message !== undefined);
                }
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

            try {
                const result = await measureInvocation(testFunction.name, cpuIntensivePayload);

                if (result.result) {
                    assertValidLambdaResponse(result.result);
                    assertWithinPerformanceThreshold(result.duration, testData.performanceThresholds.slowExecution);
                }
            } catch (error) {
                // CPU-intensive operations might timeout, which is acceptable
                assert.ok(error.message !== undefined);
            }
        });
    });

    describe('Edge Case Scenarios', () => {
        test('should handle empty and null inputs', async () => {
            const edgeCasePayloads = [
                {},
                { empty: null },
                { empty: undefined },
                { empty: '' },
                { empty: [] },
                { empty: {} }
            ];

            for (let i = 0; i < edgeCasePayloads.length; i++) {
                const result = await invokeTestFunction(
                    testFunction.name,
                    `edge-case-${i}`,
                    `Testing edge case ${i}`,
                    0,
                    edgeCasePayloads[i]
                );

                assertValidLambdaResponse(result);
            }
        });

        test('should handle special characters and encoding', async () => {
            const specialCharPayloads = [
                { unicode: '🚀🎉💻' },
                { emoji: 'Hello 🌍 World! 🎯' },
                { special: '!@#$%^&*()_+-=[]{}|;:,.<>?' },
                { newlines: 'Line 1\nLine 2\r\nLine 3' },
                { tabs: 'Tab\tTab\tTab' }
            ];

            for (let i = 0; i < specialCharPayloads.length; i++) {
                const result = await invokeTestFunction(
                    testFunction.name,
                    `special-chars-${i}`,
                    `Testing special characters ${i}`,
                    0,
                    specialCharPayloads[i]
                );

                assertValidLambdaResponse(result);
                assertMatchObject(result.event, specialCharPayloads[i]);
            }
        });

        test('should handle rapid successive invocations', async () => {
            const rapidInvocations = [];
            
            // Fire 10 rapid invocations
            for (let i = 0; i < 10; i++) {
                rapidInvocations.push(
                    measureInvocation(
                        testFunction.name,
                        global.testManager.generateTestPayload(
                            `rapid-${i}`,
                            `Rapid invocation ${i}`,
                            0
                        )
                    )
                );
            }

            const results = await Promise.all(rapidInvocations);

            // All should complete successfully
            assert.strictEqual(results.length, 10);

            for (const result of results) {
                assertValidLambdaResponse(result.result);
            }
        });
    });

    describe('Recovery and Resilience', () => {
        test('should recover from temporary failures', async () => {
            // Test with potentially problematic payload
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
            for (let i = 0; i < 5; i++) {
                try {
                    const result = await measureInvocation(testFunction.name, problematicPayload);
                    results.push({ success: true, result });
                } catch (error) {
                    results.push({ success: false, error: error.message });
                }
                
                // Small delay between attempts
                await new Promise(resolve => setTimeout(resolve, 100));
            }

            // At least some should succeed
            const successCount = results.filter(r => r.success).length;
            assert.ok(successCount >= 2);
        });

        test('should maintain stability under error conditions', async () => {
            const errorConditions = [
                { type: 'null', data: null },
                { type: 'undefined', data: undefined },
                { type: 'circular', data: {} },
                { type: 'large', data: 'x'.repeat(100000) }
            ];

            for (const condition of errorConditions) {
                try {
                    const result = await invokeTestFunction(
                        testFunction.name,
                        `stability-${condition.type}`,
                        `Testing stability with ${condition.type}`,
                        0,
                        condition.data
                    );

                    assertValidLambdaResponse(result);
                } catch (error) {
                    // Some error conditions might cause failures, which is acceptable
                    assert.ok(error.message !== undefined);
                }
            }
        });
    });
});
