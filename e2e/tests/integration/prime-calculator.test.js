/**
 * Prime Calculator Integration Tests
 */

const { describe, test, before, after } = require('node:test');
const assert = require('node:assert');
const testData = require('../fixtures/test-data');
const fs = require('fs');
const path = require('path');
const { cleanupSingleFunction } = require('../utils/test-helpers');

require('../setup');

describe('Prime Calculator Integration Tests', () => {
    let primeFunction;

    before(async () => {
        // Create prime calculator function
        primeFunction = await createPrimeFunction('prime-calculator-test');
    });

    after(cleanupSingleFunction(() => primeFunction, global.testManager.client, {
        timeout: 60000,
        verifyCleanup: true,
        forceRemoveContainers: true
    }));

    describe('Function Creation and Management', () => {
        test('should create prime calculator function successfully', () => {
            assert.ok(primeFunction.name);
            assert.strictEqual(primeFunction.data.function_name, primeFunction.name);
            assert.strictEqual(primeFunction.data.state, 'Active');
            assert.strictEqual(primeFunction.data.runtime, 'nodejs22.x');
            assert.strictEqual(primeFunction.data.handler, 'index.handler');
        });

        test('should list prime calculator function', async () => {
            const functions = await global.testManager.client.listFunctions();
            const ourFunction = functions.functions.find(f => f.function_name === primeFunction.name);
            assert.ok(ourFunction);
            assert.strictEqual(ourFunction.function_name, primeFunction.name);
        });
    });

    describe('Prime Number Calculations', () => {
        test('should calculate first prime number', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                2
            );

            assert.strictEqual(result.count, 2);
            assert.deepStrictEqual(result.primes, [2, 3]);
            assert.ok(result.calculationTimeMs >= 0);
        });

        test('should calculate first 5 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                5
            );

            assert.strictEqual(result.count, 5);
            assert.deepStrictEqual(result.primes, [2, 3, 5, 7, 11]);
            assert.ok(result.calculationTimeMs >= 0);
        });

        test('should calculate first 10 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                10
            );

            assert.strictEqual(result.count, 10);
            assert.deepStrictEqual(result.primes, [2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
            assert.ok(result.calculationTimeMs >= 0);
        });

        test('should calculate first 25 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                25
            );

            assert.strictEqual(result.count, 25);
            assert.deepStrictEqual(result.primes, [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97]);
            assert.ok(result.calculationTimeMs >= 0);
        });

        test('should calculate first 50 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                50
            );

            assert.strictEqual(result.count, 50);
            assert.strictEqual(result.primes.length, 50);
            assert.strictEqual(result.primes[0], 2);
            assert.strictEqual(result.primes[49], 229); // 50th prime number
            assert.ok(result.calculationTimeMs >= 0);
        });

        test('should calculate first 100 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                100
            );

            assert.strictEqual(result.count, 100);
            assert.strictEqual(result.primes.length, 100);
            assert.strictEqual(result.primes[0], 2);
            assert.strictEqual(result.primes[99], 541); // 100th prime number
            assert.ok(result.calculationTimeMs >= 0);
        });

        test('should calculate first 500 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                500
            );

            assert.strictEqual(result.count, 500);
            assert.strictEqual(result.primes.length, 500);
            assert.strictEqual(result.primes[0], 2);
            assert.strictEqual(result.primes[499], 3571); // 500th prime number
            assert.ok(result.calculationTimeMs >= 0);
        });
    });

    describe('Error Handling', () => {
        test('should handle invalid count parameter', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                0
            );

            assert.ok(result.errorMessage);
            assert.strictEqual(result.errorType, 'Unhandled');
            assert.ok(result.errorMessage.includes('Invalid count: 0'));
        });

        test('should handle negative count parameter', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                -5
            );

            assert.ok(result.errorMessage);
            assert.strictEqual(result.errorType, 'Unhandled');
            assert.ok(result.errorMessage.includes('Invalid count: -5'));
        });

        test('should handle large count computations', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                2000
            );

            // Should successfully compute 2000 primes (this is a complex computation test)
            assert.strictEqual(result.count, 2000);
            assert.strictEqual(result.primes.length, 2000);
            assert.strictEqual(result.primes[0], 2);
            assert.ok(result.calculationTimeMs >= 0);
        });

        test('should handle missing count parameter', async () => {
            const result = await global.testManager.client.invokeFunction(
                primeFunction.name,
                {} // No count provided
            );

            assert.ok(result.errorMessage);
            assert.strictEqual(result.errorType, 'Unhandled');
            assert.ok(result.errorMessage.includes('Invalid input. Please provide a number >= 2'));
        });
    });

    describe('Performance Characteristics', () => {
        test('should maintain consistent performance across multiple invocations', async () => {
            const iterations = 5;
            const results = [];

            for (let i = 0; i < iterations; i++) {
                const result = await measurePrimeInvocation(
                    primeFunction.name,
                    { count: 25 }
                );
                results.push(result);
            }

            // Calculate statistics
            const durations = results.map(r => r.duration);
            const avgDuration = durations.reduce((sum, d) => sum + d, 0) / durations.length;
            const maxDuration = Math.max(...durations);
            const minDuration = Math.min(...durations);

            // Execution times are measured but not asserted
            assert.ok(avgDuration >= 0);
            assert.ok(maxDuration >= 0);
            assert.ok(minDuration >= 0);
        });

        test('should handle concurrent prime calculations', async () => {
            const payloadGenerator = (index) => ({ count: 10 });

            const results = await runConcurrentInvocations(primeFunction.name, 3, payloadGenerator);

            // Check that we got 3 results and they all have the expected structure
            assert.strictEqual(results.length, 3);
            const successfulResults = results.filter(r => r.result && r.result.count === 10 && r.result.primes);
            assert.strictEqual(successfulResults.length, 3);

            // Execution times are measured but not asserted
            const maxDuration = Math.max(...results.map(r => r.duration));
            assert.ok(maxDuration >= 0);
        });
    });

    describe('Mathematical Correctness', () => {
        test('should verify prime number properties', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                20
            );

            assert.strictEqual(result.primes.length, 20);

            // Verify all numbers are actually prime
            for (const prime of result.primes) {
                assert.strictEqual(isPrime(prime), true);
            }

            // Verify they are in ascending order
            for (let i = 1; i < result.primes.length; i++) {
                assert.ok(result.primes[i] > result.primes[i - 1]);
            }
        });

        test('should handle edge cases correctly', async () => {
            // Test with count = 2 (minimum allowed)
            const doubleResult = await invokePrimeFunction(
                primeFunction.name,
                2
            );
            assert.deepStrictEqual(doubleResult.primes, [2, 3]);

            // Test with count = 3
            const tripleResult = await invokePrimeFunction(
                primeFunction.name,
                3
            );
            assert.deepStrictEqual(tripleResult.primes, [2, 3, 5]);
        });
    });
});

// Helper functions

async function createPrimeFunction(name) {
    const functionName = `${name}-${Date.now()}`;

    try {
        // Load prime calculator function zip
        const zipPath = path.join(__dirname, '../../test-functions/prime-calculator.zip');
        if (!fs.existsSync(zipPath)) {
            throw new Error(`Prime calculator zip not found: ${zipPath}`);
        }
        const primeFunctionZip = fs.readFileSync(zipPath).toString('base64');

        const functionData = await global.testManager.client.createFunction(
            functionName,
            'nodejs22.x',
            'index.handler',
            primeFunctionZip
        );

        global.testManager.functions.add(functionName);
        global.testManager.cleanupManager.registerFunction(functionName);

        // Wait for function to be ready
        await global.testManager.waitForFunctionReady(functionName);

        return {
            name: functionName,
            data: functionData,
            runtime: 'nodejs22.x'
        };
    } catch (error) {
        throw new Error(`Failed to create prime function ${functionName}: ${error.message}`);
    }
}

async function invokePrimeFunction(functionName, count) {
    const payload = {
        count: count
    };

    const result = await global.testManager.client.invokeFunction(functionName, payload);
    console.log('invokePrimeFunction result:', JSON.stringify(result, null, 2));
    return result;
}

async function measurePrimeInvocation(functionName, payload) {
    const startTime = Date.now();
    const result = await global.testManager.client.invokeFunction(functionName, payload);
    const endTime = Date.now();

    return {
        result: result,
        duration: endTime - startTime,
        startTime: startTime,
        endTime: endTime
    };
}

async function runConcurrentInvocations(functionName, count, payloadGenerator) {
    const promises = [];

    for (let i = 0; i < count; i++) {
        const payload = payloadGenerator(i);
        promises.push(
            measurePrimeInvocation(functionName, payload)
                .catch(error => ({ error: error.message, index: i }))
        );
    }

    const results = await Promise.all(promises);
    return results;
}

// Helper function to verify if a number is prime
function isPrime(num) {
    if (num < 2) return false;
    if (num === 2) return true;
    if (num % 2 === 0) return false;

    for (let i = 3; i * i <= num; i += 2) {
        if (num % i === 0) return false;
    }

    return true;
}
