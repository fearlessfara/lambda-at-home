/**
 * Test Helpers - Standardized setup/teardown utilities for e2e tests
 *
 * Provides:
 * - Standard setup/teardown functions with proper timeout handling
 * - Helper functions for test lifecycle management
 * - Container monitoring utilities
 */

const CleanupManager = require('./cleanup-manager');

/**
 * Create a standardized afterAll hook with proper cleanup and timeout handling
 *
 * Usage:
 *   const { cleanupAfterAll } = require('../utils/test-helpers');
 *   afterAll(cleanupAfterAll(functionArray, client, { timeout: 90000 }));
 */
function cleanupAfterAll(functionsArray, client, options = {}) {
    return async () => {
        const timeout = options.timeout || 60000;
        const cleanupManager = new CleanupManager(client, {
            verbose: options.verbose || false,
            cleanupTimeoutMs: timeout
        });

        // Register all functions
        for (const func of functionsArray) {
            if (func && func.name) {
                cleanupManager.registerFunction(func.name);
            }
        }

        // Perform cleanup
        const result = await cleanupManager.cleanup({
            parallel: options.parallel !== false,
            verifyCleanup: options.verifyCleanup !== false,
            forceRemoveContainers: options.forceRemoveContainers !== false
        });

        if (!result.success && options.throwOnFailure) {
            throw new Error(`Cleanup failed: ${result.failed} function(s) could not be deleted`);
        }

        return result;
    };
}

/**
 * Create a standardized afterAll hook for function name arrays
 *
 * Usage:
 *   const { cleanupFunctionsByName } = require('../utils/test-helpers');
 *   afterAll(cleanupFunctionsByName(functionNamesArray, client));
 */
function cleanupFunctionsByName(functionNames, client, options = {}) {
    return async () => {
        const timeout = options.timeout || 60000;
        const cleanupManager = new CleanupManager(client, {
            verbose: options.verbose || false,
            cleanupTimeoutMs: timeout
        });

        // Register all function names
        for (const name of functionNames) {
            if (name) {
                cleanupManager.registerFunction(name);
            }
        }

        // Perform cleanup
        const result = await cleanupManager.cleanup({
            parallel: options.parallel !== false,
            verifyCleanup: options.verifyCleanup !== false,
            forceRemoveContainers: options.forceRemoveContainers !== false
        });

        if (!result.success && options.throwOnFailure) {
            throw new Error(`Cleanup failed: ${result.failed} function(s) could not be deleted`);
        }

        return result;
    };
}

/**
 * Create a cleanup function for a single function
 *
 * Usage:
 *   afterAll(cleanupSingleFunction(() => myFunction, client));
 *   or
 *   afterAll(cleanupSingleFunction('my-function-name', client));
 */
function cleanupSingleFunction(functionNameOrObjectOrGetter, client, options = {}) {
    return async () => {
        // Handle getter function
        let functionObj = typeof functionNameOrObjectOrGetter === 'function'
            ? functionNameOrObjectOrGetter()
            : functionNameOrObjectOrGetter;

        const functionName = typeof functionObj === 'string'
            ? functionObj
            : functionObj?.name;

        if (!functionName) {
            console.warn('⚠️ No function name provided for cleanup');
            return;
        }

        const timeout = options.timeout || 60000;
        const cleanupManager = new CleanupManager(client, {
            verbose: options.verbose || false,
            cleanupTimeoutMs: timeout
        });

        cleanupManager.registerFunction(functionName);

        const result = await cleanupManager.cleanup({
            verifyCleanup: options.verifyCleanup !== false,
            forceRemoveContainers: options.forceRemoveContainers !== false
        });

        if (!result.success && options.throwOnFailure) {
            throw new Error(`Failed to cleanup function ${functionName}`);
        }

        if (client && typeof client.close === 'function') {
            client.close();
        }

        return result;
    };
}

/**
 * Create a cleanup function that handles temp files
 *
 * Usage:
 *   afterAll(cleanupWithTempFiles(functionArray, client, tempFilePaths));
 */
function cleanupWithTempFiles(functionsArray, client, tempFilePaths = [], options = {}) {
    return async () => {
        const fs = require('fs');

        // Clean up temp files first
        for (const filePath of tempFilePaths) {
            try {
                if (filePath && fs.existsSync(filePath)) {
                    fs.unlinkSync(filePath);
                    if (options.verbose) {
                        console.log(`🗑️ Cleaned up temp file: ${filePath}`);
                    }
                }
            } catch (error) {
                console.warn(`⚠️ Failed to delete temp file ${filePath}: ${error.message}`);
            }
        }

        // Then clean up functions
        const cleanup = cleanupAfterAll(functionsArray, client, options);
        return await cleanup();
    };
}

/**
 * Wrapper to add timeout to beforeAll
 *
 * Usage:
 *   beforeAll(withTimeout(async () => { ... }, 30000));
 */
function withTimeout(fn, timeoutMs = 30000) {
    return async () => {
        const timeoutPromise = new Promise((_, reject) => {
            setTimeout(() => reject(new Error(`beforeAll timeout after ${timeoutMs}ms`)), timeoutMs);
        });

        return Promise.race([fn(), timeoutPromise]);
    };
}

/**
 * Standardized function registration helper
 * Automatically registers function with cleanup on creation
 */
function registerFunction(functionObj, functionsArray, cleanupManager = null) {
    if (functionObj && functionObj.name) {
        functionsArray.push(functionObj);
        if (cleanupManager) {
            cleanupManager.registerFunction(functionObj.name);
        }
    }
    return functionObj;
}

/**
 * Container assertion helpers for tests
 */
const containerAssertions = {
    /**
     * Assert that container count matches expected value
     */
    async assertContainerCount(functionName, expectedCount, DockerUtils) {
        const actualCount = DockerUtils.getContainerCount(functionName);
        if (actualCount !== expectedCount) {
            throw new Error(
                `Expected ${expectedCount} container(s) for ${functionName}, but found ${actualCount}`
            );
        }
        return actualCount;
    },

    /**
     * Assert that container count is within a range
     */
    async assertContainerCountInRange(functionName, minCount, maxCount, DockerUtils) {
        const actualCount = DockerUtils.getContainerCount(functionName);
        if (actualCount < minCount || actualCount > maxCount) {
            throw new Error(
                `Expected ${minCount}-${maxCount} container(s) for ${functionName}, but found ${actualCount}`
            );
        }
        return actualCount;
    },

    /**
     * Assert that containers are eventually cleaned up
     */
    async assertContainersCleanedUp(functionName, timeoutMs = 30000, DockerUtils) {
        const startTime = Date.now();
        let containerCount = DockerUtils.getContainerCount(functionName);

        while (containerCount > 0 && Date.now() - startTime < timeoutMs) {
            await new Promise(resolve => setTimeout(resolve, 2000));
            containerCount = DockerUtils.getContainerCount(functionName);
        }

        if (containerCount > 0) {
            throw new Error(
                `Expected all containers for ${functionName} to be cleaned up, but found ${containerCount} after ${timeoutMs}ms`
            );
        }

        return true;
    }
};

module.exports = {
    cleanupAfterAll,
    cleanupFunctionsByName,
    cleanupSingleFunction,
    cleanupWithTempFiles,
    withTimeout,
    registerFunction,
    containerAssertions
};
