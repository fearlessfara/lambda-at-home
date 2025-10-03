/**
 * Node.js Dependencies Integration Tests
 * 
 * Tests that verify node_modules with actual npm dependencies are loaded correctly
 * by the Lambda runtime environment.
 */

const { describe, test, before, after } = require('node:test');
const assert = require('node:assert');
const {
    assertValidLambdaResponse,
    assertWithinPerformanceThreshold,
    assertSuccessfulInvocations,
    assertMatchObject
} = require('../utils/assertions');
const { cleanupSingleFunction, cleanupAfterAll, cleanupWithTempFiles } = require('../utils/test-helpers');

require('../setup');

const testData = require('../fixtures/test-data');
const fs = require('fs');
const path = require('path');

describe('Lambda@Home Node.js Dependencies Tests', () => {
    let testFunctions = [];
    let depsTestZip = null;

    before(async () => {
        // Load the test function with dependencies
        const zipPath = path.join(__dirname, '../../lambda-deps-test.zip');
        if (!fs.existsSync(zipPath)) {
            throw new Error(`Dependencies test zip not found: ${zipPath}`);
        }
        depsTestZip = fs.readFileSync(zipPath).toString('base64');
    });

    after(async () => {
        for (const testFunction of testFunctions) {
            await global.testManager.client.deleteFunction(testFunction.name);
        }
    });

    describe('Basic Dependencies Loading', () => {
        test('should load and use lodash dependency correctly', async () => {
            const testFunction = await createDepsTestFunction('lodash-test');
            testFunctions.push(testFunction);

            const result = await invokeDepsTestFunction(
                testFunction.name,
                'lodash-test',
                'Testing lodash functionality',
                { input: 'hello world' }
            );

            assertValidLambdaResponse(result);
            assert.strictEqual(result.success, true);
            assert.ok(result.lodash !== undefined);
            assert.deepStrictEqual(result.lodash.original, [1, 2, 3, 4, 5]);
            assert.deepStrictEqual(result.lodash.doubled, [2, 4, 6, 8, 10]);
            assert.strictEqual(result.lodash.sum, 15);
            assert.deepStrictEqual(result.lodash.chunked, [[1, 2], [3, 4], [5]]);
            assert.strictEqual(result.lodash.processedInput, 'HELLO WORLD');
            assert.strictEqual(result.validation.lodashWorking, true);
        });

        test('should load and use moment dependency correctly', async () => {
            const testFunction = await createDepsTestFunction('moment-test');
            testFunctions.push(testFunction);

            const result = await invokeDepsTestFunction(
                testFunction.name,
                'moment-test',
                'Testing moment functionality',
                {}
            );

            assertValidLambdaResponse(result);
            assert.strictEqual(result.success, true);
            assert.ok(result.moment !== undefined);
            assert.strictEqual(typeof result.moment.currentTime, 'string');
            assert.match(result.moment.currentTime, /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
            assert.strictEqual(typeof result.moment.unixTimestamp, 'number');
            assert.strictEqual(result.moment.isAfterYesterday, true);
            assert.strictEqual(result.validation.momentWorking, true);
        });

        test('should load and use uuid dependency correctly', async () => {
            const testFunction = await createDepsTestFunction('uuid-test');
            testFunctions.push(testFunction);

            const result = await invokeDepsTestFunction(
                testFunction.name,
                'uuid-test',
                'Testing uuid functionality',
                {}
            );

            assertValidLambdaResponse(result);
            assert.strictEqual(result.success, true);
            assert.ok(result.uuid !== undefined);
            assert.strictEqual(typeof result.uuid.generated, 'string');
            assert.strictEqual(result.uuid.isValid, true);
            assert.match(result.uuid.generated, /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
            assert.strictEqual(result.validation.uuidWorking, true);
        });
    });

    describe('Multiple Dependencies Integration', () => {
        test('should load and use all dependencies together', async () => {
            const testFunction = await createDepsTestFunction('all-deps-test');
            testFunctions.push(testFunction);

            const result = await invokeDepsTestFunction(
                testFunction.name,
                'all-deps-test',
                'Testing all dependencies together',
                { input: 'integration test' }
            );

            assertValidLambdaResponse(result);
            assert.strictEqual(result.success, true);
            
            // Verify all dependencies are working
            assert.strictEqual(result.validation.allDependenciesLoaded, true);
            assert.strictEqual(result.validation.lodashWorking, true);
            assert.strictEqual(result.validation.momentWorking, true);
            assert.strictEqual(result.validation.uuidWorking, true);
            
            // Verify specific functionality
            assert.strictEqual(result.lodash.processedInput, 'INTEGRATION TEST');
            assert.match(result.moment.currentTime, /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
            assert.strictEqual(result.uuid.isValid, true);
        });

        test('should handle dependency errors gracefully', async () => {
            const testFunction = await createDepsTestFunction('error-handling-test');
            testFunctions.push(testFunction);

            // Test with invalid input that might cause issues
            const result = await invokeDepsTestFunction(
                testFunction.name,
                'error-test',
                'Testing error handling with dependencies',
                { input: null }
            );

            // Should still work even with null input
            assertValidLambdaResponse(result);
            assert.strictEqual(result.success, true);
            assert.strictEqual(result.validation.allDependenciesLoaded, true);
        });
    });

    describe('Runtime Compatibility', () => {
        for (const runtime of testData.runtimes) {
            test(`should work with ${runtime.name} runtime`, async () => {
                const testFunction = await createDepsTestFunction(`deps-${runtime.name.replace('.', '-')}`, runtime.name);
                testFunctions.push(testFunction);

                const result = await invokeDepsTestFunction(
                    testFunction.name,
                    `runtime-${runtime.name}`,
                    `Testing dependencies with ${runtime.name}`,
                    { input: 'runtime test' }
                );

                assertValidLambdaResponse(result);
                assert.strictEqual(result.success, true);
                assert.strictEqual(result.nodeVersion, runtime.version);
                assert.strictEqual(result.runtime, 'node');
                assert.strictEqual(result.validation.allDependenciesLoaded, true);
            });
        }
    });

    describe('Performance with Dependencies', () => {
        test('should maintain good performance with dependencies loaded', async () => {
            const testFunction = await createDepsTestFunction('perf-deps-test');
            testFunctions.push(testFunction);

            // Warm up the function
            await invokeDepsTestFunction(
                testFunction.name,
                'warmup',
                'Warmup invocation',
                {}
            );

            const iterations = 3;
            const results = [];

            for (let i = 0; i < iterations; i++) {
                const result = await measureInvocation(
                    testFunction.name,
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
                assertValidLambdaResponse(result.result);
                assert.strictEqual(result.result.success, true);
                assert.strictEqual(result.result.validation.allDependenciesLoaded, true);
            }

            // Performance should be reasonable (allowing more time for dependency loading)
            const durations = results.map(r => r.duration);
            const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
            assert.ok(avgDuration < 1000); // 1 second threshold for dependency-loaded functions
        });
    });

    describe('Concurrent Dependencies Usage', () => {
        test('should handle concurrent invocations with dependencies', async () => {
            const testFunction = await createDepsTestFunction('concurrent-deps-test');
            testFunctions.push(testFunction);

            const concurrentCount = 3;
            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    `concurrent-deps-${index}`,
                    `Concurrent dependencies test ${index}`,
                    0
                );

            const results = await runConcurrentInvocations(testFunction.name, concurrentCount, payloadGenerator);
            
            assertSuccessfulInvocations(results, concurrentCount);
            
            // All results should have dependencies working
            for (const result of results) {
                assert.strictEqual(result.result.success, true);
                assert.strictEqual(result.result.validation.allDependenciesLoaded, true);
                assert.strictEqual(result.result.validation.lodashWorking, true);
                assert.strictEqual(result.result.validation.momentWorking, true);
                assert.strictEqual(result.result.validation.uuidWorking, true);
            }
        });
    });

    describe('Dependency Version Compatibility', () => {
        test('should work with different Node.js versions and dependency versions', async () => {
            const testFunction = await createDepsTestFunction('version-compat-test');
            testFunctions.push(testFunction);

            const result = await invokeDepsTestFunction(
                testFunction.name,
                'version-test',
                'Testing version compatibility',
                { input: 'version test' }
            );

            assertValidLambdaResponse(result);
            assert.strictEqual(result.success, true);
            assert.strictEqual(result.validation.allDependenciesLoaded, true);
            
            // Verify the specific versions we're using work correctly
            assert.strictEqual(result.lodash.sum, 15); // lodash 4.17.21
            assert.match(result.moment.currentTime, /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/); // moment 2.29.4
            assert.strictEqual(result.uuid.isValid, true); // uuid 9.0.1
        });
    });

    // Helper functions
    async function createDepsTestFunction(name, runtime = 'nodejs22.x') {
        const functionName = `${name}-${Date.now()}`;
        
        try {
            const functionData = await global.testManager.client.createFunction(
                functionName,
                runtime,
                'index.handler',
                depsTestZip
            );
            
            // Wait for function to be ready
            await global.testManager.waitForFunctionReady(functionName);
            
            return {
                name: functionName,
                data: functionData,
                runtime: runtime
            };
        } catch (error) {
            throw new Error(`Failed to create dependencies test function ${functionName}: ${error.message}`);
        }
    }

    async function invokeDepsTestFunction(functionName, testId, message, input = {}) {
        const payload = {
            testId: testId,
            message: message,
            input: input.input || 'default input',
            wait: input.wait || 0
        };

        const result = await global.testManager.client.invokeFunction(functionName, payload);
        return result;
    }
});
