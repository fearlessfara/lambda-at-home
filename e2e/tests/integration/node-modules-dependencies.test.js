/**
 * Node.js Dependencies Integration Tests
 * 
 * Tests that verify node_modules with actual npm dependencies are loaded correctly
 * by the Lambda runtime environment.
 */

const testData = require('../fixtures/test-data');
const fs = require('fs');
const path = require('path');

describe('Lambda@Home Node.js Dependencies Tests', () => {
    let testFunctions = [];
    let depsTestZip = null;

    beforeAll(async () => {
        // Load the test function with dependencies
        const zipPath = path.join(__dirname, '../../lambda-deps-test.zip');
        if (!fs.existsSync(zipPath)) {
            throw new Error(`Dependencies test zip not found: ${zipPath}`);
        }
        depsTestZip = fs.readFileSync(zipPath).toString('base64');
    });

    afterAll(async () => {
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

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.lodash).toBeDefined();
            expect(result.lodash.original).toEqual([1, 2, 3, 4, 5]);
            expect(result.lodash.doubled).toEqual([2, 4, 6, 8, 10]);
            expect(result.lodash.sum).toBe(15);
            expect(result.lodash.chunked).toEqual([[1, 2], [3, 4], [5]]);
            expect(result.lodash.processedInput).toBe('HELLO WORLD');
            expect(result.validation.lodashWorking).toBe(true);
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

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.moment).toBeDefined();
            expect(typeof result.moment.currentTime).toBe('string');
            expect(result.moment.currentTime).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
            expect(typeof result.moment.unixTimestamp).toBe('number');
            expect(result.moment.isAfterYesterday).toBe(true);
            expect(result.validation.momentWorking).toBe(true);
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

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.uuid).toBeDefined();
            expect(typeof result.uuid.generated).toBe('string');
            expect(result.uuid.isValid).toBe(true);
            expect(result.uuid.generated).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
            expect(result.validation.uuidWorking).toBe(true);
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

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            
            // Verify all dependencies are working
            expect(result.validation.allDependenciesLoaded).toBe(true);
            expect(result.validation.lodashWorking).toBe(true);
            expect(result.validation.momentWorking).toBe(true);
            expect(result.validation.uuidWorking).toBe(true);
            
            // Verify specific functionality
            expect(result.lodash.processedInput).toBe('INTEGRATION TEST');
            expect(result.moment.currentTime).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
            expect(result.uuid.isValid).toBe(true);
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
            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.validation.allDependenciesLoaded).toBe(true);
        });
    });

    describe('Runtime Compatibility', () => {
        test.each(testData.runtimes)('should work with $name runtime', async (runtime) => {
            const testFunction = await createDepsTestFunction(`deps-${runtime.name.replace('.', '-')}`, runtime.name);
            testFunctions.push(testFunction);

            const result = await invokeDepsTestFunction(
                testFunction.name,
                `runtime-${runtime.name}`,
                `Testing dependencies with ${runtime.name}`,
                { input: 'runtime test' }
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.nodeVersion).toBe(runtime.version);
            expect(result.runtime).toBe('node');
            expect(result.validation.allDependenciesLoaded).toBe(true);
        });
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
                expect(result.result).toBeValidLambdaResponse();
                expect(result.result.success).toBe(true);
                expect(result.result.validation.allDependenciesLoaded).toBe(true);
            }

            // Performance should be reasonable (allowing more time for dependency loading)
            const durations = results.map(r => r.duration);
            const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
            expect(avgDuration).toBeLessThan(1000); // 1 second threshold for dependency-loaded functions
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
            
            expect(results).toHaveSuccessfulInvocations(concurrentCount);
            
            // All results should have dependencies working
            for (const result of results) {
                expect(result.result.success).toBe(true);
                expect(result.result.validation.allDependenciesLoaded).toBe(true);
                expect(result.result.validation.lodashWorking).toBe(true);
                expect(result.result.validation.momentWorking).toBe(true);
                expect(result.result.validation.uuidWorking).toBe(true);
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

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.validation.allDependenciesLoaded).toBe(true);
            
            // Verify the specific versions we're using work correctly
            expect(result.lodash.sum).toBe(15); // lodash 4.17.21
            expect(result.moment.currentTime).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/); // moment 2.29.4
            expect(result.uuid.isValid).toBe(true); // uuid 9.0.1
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
