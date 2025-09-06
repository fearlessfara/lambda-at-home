/**
 * Test Data Fixtures
 */

const testData = {
    runtimes: [
        { name: 'nodejs18.x', version: 'v18.20.8' },
        { name: 'nodejs22.x', version: 'v22.19.0' }
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

    apiGatewayRoutes: [
        { path: '/test-route', method: 'POST', description: 'Basic POST route' },
        { path: '/test-route', method: 'GET', description: 'Basic GET route' },
        { path: '/test-route', method: 'PUT', description: 'Basic PUT route' },
        { path: '/test-route', method: 'DELETE', description: 'Basic DELETE route' }
    ],

    expectedResponses: {
        success: {
            success: true,
            nodeVersion: expect.stringMatching(/^v\d+\.\d+\.\d+$/),
            runtime: 'node',
            timestamp: expect.stringMatching(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/)
        },
        error: {
            errorMessage: expect.any(String),
            errorType: expect.any(String)
        }
    },

    performanceThresholds: {
        fastExecution: 150, // ms - increased for real-world performance
        mediumExecution: 1600, // ms - increased for memory-intensive operations
        slowExecution: 2000, // ms
        concurrentExecution: 2000, // ms for 5 concurrent
        memoryUsage: 100 * 1024 * 1024, // 100MB
        cpuUsage: 80 // percentage
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
