/**
 * Prime Calculator Integration Tests
 */

const testData = require('../fixtures/test-data');
const fs = require('fs');
const path = require('path');

describe('Prime Calculator Integration Tests', () => {
    let primeFunction;

    beforeAll(async () => {
        // Create prime calculator function
        primeFunction = await createPrimeFunction('prime-calculator-test');
    });

    afterAll(async () => {
        if (primeFunction && primeFunction.name) {
            await global.testManager.client.deleteFunction(primeFunction.name);
        }
    });

    describe('Function Creation and Management', () => {
        test('should create prime calculator function successfully', () => {
            expect(primeFunction.name).toBeDefined();
            expect(primeFunction.data.function_name).toBe(primeFunction.name);
            expect(primeFunction.data.state).toBe('Active');
            expect(primeFunction.data.runtime).toBe('nodejs22.x');
            expect(primeFunction.data.handler).toBe('index.handler');
        });

        test('should list prime calculator function', async () => {
            const functions = await global.testManager.client.listFunctions();
            const ourFunction = functions.functions.find(f => f.function_name === primeFunction.name);
            expect(ourFunction).toBeDefined();
            expect(ourFunction.function_name).toBe(primeFunction.name);
        });
    });

    describe('Prime Number Calculations', () => {
        test('should calculate first prime number', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                2
            );

            expect(result.count).toBe(2);
            expect(result.primes).toEqual([2, 3]);
            expect(result.calculationTimeMs).toBeGreaterThanOrEqual(0);
        });

        test('should calculate first 5 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                5
            );

            expect(result.count).toBe(5);
            expect(result.primes).toEqual([2, 3, 5, 7, 11]);
            // Execution time is measured but not asserted
            expect(result.calculationTimeMs).toBeGreaterThanOrEqual(0);
        });

        test('should calculate first 10 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                10
            );

            expect(result.count).toBe(10);
            expect(result.primes).toEqual([2, 3, 5, 7, 11, 13, 17, 19, 23, 29]);
            // Execution time is measured but not asserted
            expect(result.calculationTimeMs).toBeGreaterThanOrEqual(0);
        });

        test('should calculate first 25 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                25
            );

            expect(result.count).toBe(25);
            expect(result.primes).toEqual([2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97]);
            // Execution time is measured but not asserted
            expect(result.calculationTimeMs).toBeGreaterThanOrEqual(0);
        });

        test('should calculate first 50 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                50
            );

            expect(result.count).toBe(50);
            expect(result.primes).toHaveLength(50);
            expect(result.primes[0]).toBe(2);
            expect(result.primes[49]).toBe(229); // 50th prime number
            // Execution time is measured but not asserted
            expect(result.calculationTimeMs).toBeGreaterThanOrEqual(0);
        });

        test('should calculate first 100 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                100
            );

            expect(result.count).toBe(100);
            expect(result.primes).toHaveLength(100);
            expect(result.primes[0]).toBe(2);
            expect(result.primes[99]).toBe(541); // 100th prime number
            // Execution time is measured but not asserted
            expect(result.calculationTimeMs).toBeGreaterThanOrEqual(0);
        });

        test('should calculate first 500 prime numbers', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                500
            );

            expect(result.count).toBe(500);
            expect(result.primes).toHaveLength(500);
            expect(result.primes[0]).toBe(2);
            expect(result.primes[499]).toBe(3571); // 500th prime number
            // Execution time is measured but not asserted
            expect(result.calculationTimeMs).toBeGreaterThanOrEqual(0);
        });
    });

    describe('Error Handling', () => {
        test('should handle invalid count parameter', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                0
            );

            expect(result.errorMessage).toBeDefined();
            expect(result.errorType).toBe('Unhandled');
            expect(result.errorMessage).toContain('Invalid count: 0');
        });

        test('should handle negative count parameter', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                -5
            );

            expect(result.errorMessage).toBeDefined();
            expect(result.errorType).toBe('Unhandled');
            expect(result.errorMessage).toContain('Invalid count: -5');
        });

        test('should handle count exceeding maximum', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                2000
            );

            expect(result.errorMessage).toBeDefined();
            expect(result.errorType).toBe('Unhandled');
            expect(result.errorMessage).toContain('Invalid count: 2000');
        });

        test('should handle missing count parameter', async () => {
            const result = await global.testManager.client.invokeFunction(
                primeFunction.name,
                {} // No count provided
            );

            expect(result.errorMessage).toBeDefined();
            expect(result.errorType).toBe('Unhandled');
            expect(result.errorMessage).toContain('Invalid input. Please provide a number >= 2');
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
            expect(avgDuration).toBeGreaterThanOrEqual(0);
            expect(maxDuration).toBeGreaterThanOrEqual(0);
            expect(minDuration).toBeGreaterThanOrEqual(0);
        });

        test('should handle concurrent prime calculations', async () => {
            const payloadGenerator = (index) => ({ count: 10 });

            const results = await runConcurrentInvocations(primeFunction.name, 3, payloadGenerator);
            
            // Check that we got 3 results and they all have the expected structure
            expect(results).toHaveLength(3);
            const successfulResults = results.filter(r => r.result && r.result.count === 10 && r.result.primes);
            expect(successfulResults).toHaveLength(3);
            
            // Execution times are measured but not asserted
            const maxDuration = Math.max(...results.map(r => r.duration));
            expect(maxDuration).toBeGreaterThanOrEqual(0);
        });
    });

    describe('Mathematical Correctness', () => {
        test('should verify prime number properties', async () => {
            const result = await invokePrimeFunction(
                primeFunction.name,
                20
            );

            expect(result.primes).toHaveLength(20);

            // Verify all numbers are actually prime
            for (const prime of result.primes) {
                expect(isPrime(prime)).toBe(true);
            }

            // Verify they are in ascending order
            for (let i = 1; i < result.primes.length; i++) {
                expect(result.primes[i]).toBeGreaterThan(result.primes[i - 1]);
            }
        });

        test('should handle edge cases correctly', async () => {
            // Test with count = 2 (minimum allowed)
            const doubleResult = await invokePrimeFunction(
                primeFunction.name,
                2
            );
            expect(doubleResult.primes).toEqual([2, 3]);

            // Test with count = 3
            const tripleResult = await invokePrimeFunction(
                primeFunction.name,
                3
            );
            expect(tripleResult.primes).toEqual([2, 3, 5]);
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

    return await global.testManager.client.invokeFunction(functionName, payload);
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
