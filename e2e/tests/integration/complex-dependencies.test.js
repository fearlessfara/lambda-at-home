/**
 * Complex Dependencies Integration Tests
 * 
 * Tests that verify complex npm dependencies (lodash, moment, uuid, axios, validator)
 * are loaded correctly and work as expected in the Lambda runtime environment.
 */

const testData = require('../fixtures/test-data');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

describe('Lambda@Home Complex Dependencies Tests', () => {
    let testFunctions = [];
    let complexDepsTestZip = null;
    let tempZipPath = null;

    beforeAll(async () => {
        // Build the complex dependencies test function from source
        const testFunctionPath = path.join(__dirname, '../../test-functions/large-deps-test');
        
        console.log(`📦 Building complex dependencies test function from source: ${testFunctionPath}`);
        
        // Step 1: Install dependencies
        console.log(`📦 Installing complex dependencies...`);
        try {
            execSync('npm install', { 
                cwd: testFunctionPath, 
                stdio: 'pipe' 
            });
            console.log(`✅ Complex dependencies installed successfully`);
        } catch (error) {
            throw new Error(`Failed to install complex dependencies: ${error.message}`);
        }

        // Step 2: Create ZIP file
        console.log(`📦 Creating ZIP file...`);
        tempZipPath = path.join(__dirname, '../../test-functions/complex-deps-test-temp.zip');
        
        try {
            // Remove existing temp zip if it exists
            if (fs.existsSync(tempZipPath)) {
                fs.unlinkSync(tempZipPath);
            }
            
            // Create ZIP with contents of the directory (not the directory itself)
            execSync(`cd "${testFunctionPath}" && zip -r "${tempZipPath}" ./*`, { 
                stdio: 'pipe' 
            });
            
            const zipSize = fs.statSync(tempZipPath).size;
            console.log(`✅ ZIP file created: ${tempZipPath} (${zipSize} bytes)`);
            
            // Step 3: Read ZIP as base64
            complexDepsTestZip = fs.readFileSync(tempZipPath).toString('base64');
            console.log(`✅ ZIP file loaded as base64 (${complexDepsTestZip.length} characters)`);
            
        } catch (error) {
            throw new Error(`Failed to create ZIP file: ${error.message}`);
        }
    });

    afterAll(async () => {
        // Clean up temp ZIP file
        if (tempZipPath && fs.existsSync(tempZipPath)) {
            fs.unlinkSync(tempZipPath);
            console.log(`🗑️ Cleaned up temp ZIP file: ${tempZipPath}`);
        }
        
        // Clean up all test functions
        for (const testFunction of testFunctions) {
            try {
                await global.testManager.client.deleteFunction(testFunction.name);
            } catch (error) {
                console.warn(`Failed to delete function ${testFunction.name}: ${error.message}`);
            }
        }
    });

    describe('Lodash Dependencies', () => {
        test('should load and use lodash dependency correctly', async () => {
            const testFunction = await createComplexDepsTestFunction('lodash-test');
            testFunctions.push(testFunction);

            const result = await invokeComplexDepsTestFunction(
                testFunction.name,
                'lodash-test',
                'Testing lodash functionality',
                { operation: 'lodash_test', input: 'hello world' }
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.operation).toBe('lodash_test');
            expect(result.result.result).toBe('Hello world');
            expect(result.validation.lodashWorking).toBe(true);
        });
    });

    describe('Validator Dependencies', () => {
        test('should load and use validator dependency correctly', async () => {
            const testFunction = await createComplexDepsTestFunction('validator-test');
            testFunctions.push(testFunction);

            const result = await invokeComplexDepsTestFunction(
                testFunction.name,
                'validator-test',
                'Testing validator functionality',
                { operation: 'validator_test', input: 'test@example.com' }
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.operation).toBe('validator_test');
            expect(result.result.result.isEmail).toBe(true);
            expect(result.result.result.isURL).toBe(true);
            expect(result.result.result.isNumeric).toBe(false); // "123" is numeric, but we're testing with "test@example.com"
            expect(result.validation.validatorWorking).toBe(true);
        });
    });

    describe('UUID Dependencies', () => {
        test('should load and use uuid dependency correctly', async () => {
            const testFunction = await createComplexDepsTestFunction('uuid-test');
            testFunctions.push(testFunction);

            const result = await invokeComplexDepsTestFunction(
                testFunction.name,
                'uuid-test',
                'Testing uuid functionality',
                { operation: 'uuid_test' }
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.operation).toBe('uuid_test');
            expect(result.result.result).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
            expect(result.validation.uuidWorking).toBe(true);
        });
    });

    describe('Axios Dependencies', () => {
        test('should load and use axios dependency correctly', async () => {
            const testFunction = await createComplexDepsTestFunction('axios-test');
            testFunctions.push(testFunction);

            const result = await invokeComplexDepsTestFunction(
                testFunction.name,
                'axios-test',
                'Testing axios functionality',
                { operation: 'axios_test' }
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.operation).toBe('axios_test');
            expect(result.result.result.status).toBe(200);
            expect(result.result.result.data).toBeDefined();
            expect(result.validation.axiosWorking).toBe(true);
        });
    });

    describe('All Complex Dependencies Integration', () => {
        test('should load and use all complex dependencies together', async () => {
            const testFunction = await createComplexDepsTestFunction('all-deps-test');
            testFunctions.push(testFunction);

            const result = await invokeComplexDepsTestFunction(
                testFunction.name,
                'all-deps-test',
                'Testing all dependencies together',
                { operation: 'all_deps_test', input: 'integration test' }
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.operation).toBe('all_deps_test');
            expect(result.result.result.lodash).toBe('Integration test');
            expect(result.result.result.uuid).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
            expect(result.result.result.moment).toBeDefined();
            expect(result.result.result.validator.isEmail).toBe(true);
            expect(result.result.result.validator.isURL).toBe(true);
            
            // Verify all dependencies are working
            expect(result.validation.allDependenciesLoaded).toBe(true);
            expect(result.validation.lodashWorking).toBe(true);
            expect(result.validation.momentWorking).toBe(true);
            expect(result.validation.uuidWorking).toBe(true);
            expect(result.validation.axiosWorking).toBe(true);
            expect(result.validation.validatorWorking).toBe(true);
        });
    });

    describe('Complex Dependencies Performance', () => {
        test('should maintain reasonable performance with complex dependencies', async () => {
            const testFunction = await createComplexDepsTestFunction('perf-complex-deps-test');
            testFunctions.push(testFunction);

            // Warm up the function
            await invokeComplexDepsTestFunction(
                testFunction.name,
                'warmup',
                'Warmup invocation',
                { operation: 'default' }
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

            // Performance should be reasonable (higher threshold for complex dependencies)
            const durations = results.map(r => r.duration);
            const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
            expect(avgDuration).toBeLessThan(2000); // 2 second threshold for complex dependency-loaded functions
        });
    });

    describe('Complex Dependencies Error Handling', () => {
        test('should handle dependency errors gracefully', async () => {
            const testFunction = await createComplexDepsTestFunction('error-complex-deps-test');
            testFunctions.push(testFunction);

            const result = await invokeComplexDepsTestFunction(
                testFunction.name,
                'error-test',
                'Testing error handling',
                { operation: 'invalid_operation' }
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.operation).toBe('invalid_operation'); // The function passes through the operation
            expect(result.validation.allDependenciesLoaded).toBe(true);
        });
    });

    describe('Complex Dependencies Runtime Compatibility', () => {
        test.each(testData.runtimes)('should work with $name runtime', async (runtime) => {
            const testFunction = await createComplexDepsTestFunction(`complex-deps-${runtime.name.replace('.', '-')}`, runtime.name);
            testFunctions.push(testFunction);

            const result = await invokeComplexDepsTestFunction(
                testFunction.name,
                `runtime-${runtime.name}`,
                `Testing complex dependencies with ${runtime.name}`,
                { operation: 'default' }
            );

            expect(result).toBeValidLambdaResponse();
            expect(result.success).toBe(true);
            expect(result.nodeVersion).toBe(runtime.version);
            expect(result.runtime).toBe('node');
            expect(result.validation.allDependenciesLoaded).toBe(true);
        });
    });

    describe('Complex Dependencies Concurrent Usage', () => {
        test('should handle concurrent invocations with complex dependencies', async () => {
            const testFunction = await createComplexDepsTestFunction('concurrent-complex-deps-test');
            testFunctions.push(testFunction);

            const concurrentCount = 3;
            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    `concurrent-complex-deps-${index}`,
                    `Concurrent complex dependencies test ${index}`,
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
                expect(result.result.validation.axiosWorking).toBe(true);
                expect(result.result.validation.validatorWorking).toBe(true);
            }
        });
    });

    // Helper functions
    async function createComplexDepsTestFunction(name, runtime = 'nodejs22.x') {
        const functionName = `${name}-${Date.now()}`;
        
        try {
            const functionData = await global.testManager.client.createFunction(
                functionName,
                runtime,
                'index.handler',
                complexDepsTestZip
            );
            
            // Wait for function to be ready
            await global.testManager.waitForFunctionReady(functionName);
            
            return {
                name: functionName,
                data: functionData,
                runtime: runtime
            };
        } catch (error) {
            throw new Error(`Failed to create complex dependencies test function ${functionName}: ${error.message}`);
        }
    }

    async function invokeComplexDepsTestFunction(functionName, testId, message, input = {}) {
        const payload = {
            testId: testId,
            message: message,
            input: input.input || 'default input',
            operation: input.operation || 'default',
            wait: input.wait || 0
        };

        const result = await global.testManager.client.invokeFunction(functionName, payload);
        return result;
    }
});