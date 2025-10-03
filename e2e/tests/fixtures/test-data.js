/**
 * Test Data Fixtures
 */

const testData = {
    runtimes: [
        { name: 'nodejs18.x', version: 'v18.20.8' },
        { name: 'nodejs22.x', version: 'v22.20.0' },
        { name: 'nodejs24.x', version: 'v24.9.0' }
    ],

    testScenarios: {
        fast: { wait: 0, description: 'Fast execution' },
        medium: { wait: 100, description: 'Medium execution' },
        slow: { wait: 500, description: 'Slow execution' },
        verySlow: { wait: 1000, description: 'Very slow execution' }
    },

    concurrencyLevels: {
        low: 3,
        medium: 5,
        high: 10
    },

    loadTestConfigs: {
        light: { count: 5, delay: 100 },
        medium: { count: 10, delay: 50 },
        heavy: { count: 20, delay: 25 }
    },

    errorScenarios: [
        { type: 'timeout', wait: 30000, description: 'Timeout scenario' },
        { type: 'invalid', payload: null, description: 'Invalid payload' },
        { type: 'large', payload: 'x'.repeat(1000000), description: 'Large payload' }
    ],

    primeCalculationScenarios: [
        { count: 1, description: 'First prime number', expectedPrimes: [2] },
        { count: 5, description: 'First 5 prime numbers', expectedPrimes: [2, 3, 5, 7, 11] },
        { count: 10, description: 'First 10 prime numbers', expectedPrimes: [2, 3, 5, 7, 11, 13, 17, 19, 23, 29] },
        { count: 25, description: 'First 25 prime numbers', expectedPrimes: [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89, 97] },
        { count: 50, description: 'First 50 prime numbers', expectedTime: 100 },
        { count: 100, description: 'First 100 prime numbers', expectedTime: 200 },
        { count: 500, description: 'First 500 prime numbers', expectedTime: 1000 }
    ],

    apiGatewayRoutes: [
        { path: '/test-route', method: 'POST', description: 'Basic POST route' },
        { path: '/test-route', method: 'GET', description: 'Basic GET route' },
        { path: '/test-route', method: 'PUT', description: 'Basic PUT route' },
        { path: '/test-route', method: 'DELETE', description: 'Basic DELETE route' }
    ],

    expectedResponses: {
        success: {
            success: true,
            nodeVersion: /^v\d+\.\d+\.\d+$/,
            runtime: 'node',
            timestamp: /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/
        },
        error: {
            errorMessage: 'string',
            errorType: 'string'
        }
    },

    performanceThresholds: {
        fastExecution: 150, // ms - increased for real-world performance
        mediumExecution: 3000, // ms - increased for memory-intensive operations
        slowExecution: 2000, // ms
        concurrentExecution: 3000, // ms for 5 concurrent
        memoryUsage: 100 * 1024 * 1024, // 100MB
        cpuUsage: 80, // percentage
        primeCalculation: {
            small: 50, // ms for first 10 primes
            medium: 200, // ms for first 100 primes
            large: 1000, // ms for first 500 primes
            veryLarge: 5000 // ms for first 1000 primes
        }
    },

    testTimeouts: {
        functionCreation: 10000, // 10 seconds
        functionInvocation: 30000, // 30 seconds
        concurrentTest: 60000, // 60 seconds
        loadTest: 120000, // 2 minutes
        cleanup: 5000 // 5 seconds
    }
};

module.exports = testData;
