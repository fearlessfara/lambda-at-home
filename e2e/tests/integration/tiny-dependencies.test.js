/**
 * Tiny Dependencies Integration Tests
 * 
 * Tests that verify small npm dependencies are loaded correctly
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
const { execSync } = require('child_process');

describe('Lambda@Home Tiny Dependencies Tests', () => {
    let testFunctions = [];
    let tinyDepsTestZip = null;
    let tempZipPath = null;

    before(async () => {
        // Build the test function with dependencies from source
        const testFunctionPath = path.join(__dirname, '../../test-functions/deps-test');
        
        console.log(`📦 Building tiny dependencies test function from source: ${testFunctionPath}`);
        
        // Step 1: Install dependencies
        console.log(`📦 Installing dependencies...`);
        try {
            execSync('npm install', { 
                cwd: testFunctionPath, 
                stdio: 'pipe' 
            });
            console.log(`✅ Dependencies installed successfully`);
        } catch (error) {
            throw new Error(`Failed to install dependencies: ${error.message}`);
        }

        // Step 2: Create ZIP file
        console.log(`📦 Creating ZIP file...`);
        tempZipPath = path.join(__dirname, '../../test-functions/tiny-deps-test-temp.zip');
        
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
            tinyDepsTestZip = fs.readFileSync(tempZipPath).toString('base64');
            console.log(`✅ ZIP file loaded as base64 (${tinyDepsTestZip.length} characters)`);
            
        } catch (error) {
            throw new Error(`Failed to create ZIP file: ${error.message}`);
        }
    });

    after(async () => {
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

    describe('Basic Dependencies Loading', () => {
        test('should load and use uuid dependency correctly', async () => {
            const testFunction = await createTinyDepsTestFunction('uuid-test');
            testFunctions.push(testFunction);

            const result = await invokeTinyDepsTestFunction(
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

    describe('Runtime Compatibility', () => {
        for (const runtime of testData.runtimes) {
            test(`should work with ${runtime.name} runtime`, async () => {
                const testFunction = await createTinyDepsTestFunction(`tiny-deps-${runtime.name.replace('.', '-')}`, runtime.name);
                testFunctions.push(testFunction);

                const result = await invokeTinyDepsTestFunction(
                    testFunction.name,
                    `runtime-${runtime.name}`,
                    `Testing tiny dependencies with ${runtime.name}`,
                    {}
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
            const testFunction = await createTinyDepsTestFunction('perf-tiny-deps-test');
            testFunctions.push(testFunction);

            // Warm up the function
            await invokeTinyDepsTestFunction(
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

            // Performance should be reasonable
            const durations = results.map(r => r.duration);
            const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
            assert.ok(avgDuration < 500); // 500ms threshold for tiny dependency-loaded functions
        });
    });

    describe('Concurrent Dependencies Usage', () => {
        test('should handle concurrent invocations with dependencies', async () => {
            const testFunction = await createTinyDepsTestFunction('concurrent-tiny-deps-test');
            testFunctions.push(testFunction);

            const concurrentCount = 3;
            const payloadGenerator = (index) => 
                global.testManager.generateConcurrentPayload(
                    index,
                    `concurrent-tiny-deps-${index}`,
                    `Concurrent tiny dependencies test ${index}`,
                    0
                );

            const results = await runConcurrentInvocations(testFunction.name, concurrentCount, payloadGenerator);
            
            assertSuccessfulInvocations(results, concurrentCount);
            
            // All results should have dependencies working
            for (const result of results) {
                assert.strictEqual(result.result.success, true);
                assert.strictEqual(result.result.validation.allDependenciesLoaded, true);
                assert.strictEqual(result.result.validation.uuidWorking, true);
            }
        });
    });

    // Helper functions
    async function createTinyDepsTestFunction(name, runtime = 'nodejs22.x') {
        const functionName = `${name}-${Date.now()}`;
        
        try {
            const functionData = await global.testManager.client.createFunction(
                functionName,
                runtime,
                'index.handler',
                tinyDepsTestZip
            );
            
            // Wait for function to be ready
            await global.testManager.waitForFunctionReady(functionName);
            
            return {
                name: functionName,
                data: functionData,
                runtime: runtime
            };
        } catch (error) {
            throw new Error(`Failed to create tiny dependencies test function ${functionName}: ${error.message}`);
        }
    }

    async function invokeTinyDepsTestFunction(functionName, testId, message, input = {}) {
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
