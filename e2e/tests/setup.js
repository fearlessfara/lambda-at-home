/**
 * Test Setup - Global test setup using Node.js native test runner
 */

const TestManager = require('./utils/test-manager');
const { before, after } = require('node:test');

// Global test manager instance
global.testManager = new TestManager();

// Global setup
before(async () => {
    try {
        await global.testManager.setup();
    } catch (error) {
        console.error('❌ Global setup failed:', error.message);
        throw error;
    }
});

// Global teardown
after(async () => {
    try {
        await global.testManager.teardown();
    } catch (error) {
        console.error('❌ Global teardown failed:', error.message);
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
