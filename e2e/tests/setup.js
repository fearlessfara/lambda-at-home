/**
 * Jest Setup - Global test setup and configuration
 */

const TestManager = require('./utils/test-manager');

// Global test manager instance
global.testManager = new TestManager();

// Global test timeout
jest.setTimeout(60000);

// Global setup
beforeAll(async () => {
    try {
        await global.testManager.setup();
    } catch (error) {
        console.error('❌ Global setup failed:', error.message);
        throw error;
    }
});

// Global teardown
afterAll(async () => {
    try {
        await global.testManager.teardown();
    } catch (error) {
        console.error('❌ Global teardown failed:', error.message);
    }
});

// Custom matchers
expect.extend({
    toBeValidLambdaResponse(received) {
        const pass = received && 
                    typeof received === 'object' &&
                    received.success === true &&
                    typeof received.testId === 'string' &&
                    typeof received.message === 'string' &&
                    typeof received.timestamp === 'string' &&
                    typeof received.nodeVersion === 'string' &&
                    received.runtime === 'node';

        if (pass) {
            return {
                message: () => `expected ${received} not to be a valid Lambda response`,
                pass: true,
            };
        } else {
            return {
                message: () => `expected ${JSON.stringify(received)} to be a valid Lambda response`,
                pass: false,
            };
        }
    },

    toBeWithinPerformanceThreshold(received, threshold) {
        const pass = received <= threshold;

        if (pass) {
            return {
                message: () => `expected ${received}ms not to be within ${threshold}ms threshold`,
                pass: true,
            };
        } else {
            return {
                message: () => `expected ${received}ms to be within ${threshold}ms threshold`,
                pass: false,
            };
        }
    },

    toHaveSuccessfulInvocations(received, expectedCount) {
        const successCount = received.filter(r => r.result && r.result.success && !r.error).length;
        const pass = successCount === expectedCount;

        if (pass) {
            return {
                message: () => `expected ${successCount} successful invocations not to equal ${expectedCount}`,
                pass: true,
            };
        } else {
            return {
                message: () => `expected ${successCount} successful invocations to equal ${expectedCount}`,
                pass: false,
            };
        }
    }
});

// Global test utilities
global.createTestFunction = async (name, runtime = 'nodejs22.x') => {
    return await global.testManager.createTestFunction(name, runtime);
};

global.invokeTestFunction = async (functionName, testId, message, waitMs = 0, additionalData = {}) => {
    return await global.testManager.invokeTestFunction(functionName, testId, message, waitMs, additionalData);
};

global.measureInvocation = async (functionName, payload) => {
    return await global.testManager.measureInvocation(functionName, payload);
};

global.runConcurrentInvocations = async (functionName, count, payloadGenerator) => {
    return await global.testManager.runConcurrentInvocations(functionName, count, payloadGenerator);
};

global.runSequentialInvocations = async (functionName, count, payloadGenerator, delayMs = 100) => {
    return await global.testManager.runSequentialInvocations(functionName, count, payloadGenerator, delayMs);
};

// Console configuration for tests
const originalConsoleLog = console.log;
const originalConsoleError = console.error;

// Suppress console output during tests unless in verbose mode
if (!process.env.VERBOSE_TESTS) {
    console.log = (...args) => {
        if (args[0] && (args[0].includes('❌') || args[0].includes('ERROR'))) {
            originalConsoleLog(...args);
        }
    };
    
    console.error = (...args) => {
        if (args[0] && (args[0].includes('❌') || args[0].includes('ERROR'))) {
            originalConsoleError(...args);
        }
    };
}
