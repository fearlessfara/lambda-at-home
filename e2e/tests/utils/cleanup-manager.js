/**
 * Cleanup Manager - Centralized cleanup utility for e2e tests
 *
 * Provides standardized setup/teardown for tests with:
 * - Automatic function and container cleanup
 * - Container monitoring and state verification
 * - Timeout handling for cleanup operations
 * - Comprehensive error handling and logging
 */

const { execSync } = require('child_process');
const DockerUtils = require('./docker-utils');

class CleanupManager {
    constructor(testClient, options = {}) {
        this.client = testClient;
        this.functions = new Set();
        this.cleanupTimeoutMs = options.cleanupTimeoutMs || 60000; // 60 seconds default
        this.cleanupRetries = options.cleanupRetries || 3;
        this.cleanupDelayMs = options.cleanupDelayMs || 1000;
        this.verbose = options.verbose || false;
        this.containerTrackingEnabled = options.containerTracking !== false; // enabled by default
    }

    /**
     * Register a function for cleanup
     */
    registerFunction(functionName) {
        this.functions.add(functionName);
        if (this.verbose) {
            console.log(`📝 Registered function for cleanup: ${functionName}`);
        }
    }

    /**
     * Unregister a function (if manually cleaned up)
     */
    unregisterFunction(functionName) {
        this.functions.delete(functionName);
        if (this.verbose) {
            console.log(`📝 Unregistered function from cleanup: ${functionName}`);
        }
    }

    /**
     * Get all Lambda containers currently running
     */
    getLambdaContainers() {
        return DockerUtils.getLambdaContainers();
    }

    /**
     * Get containers for a specific function
     */
    getFunctionContainers(functionName) {
        const count = DockerUtils.getContainerCount(functionName);
        if (this.verbose) {
            console.log(`📊 Function ${functionName} has ${count} container(s)`);
        }
        return count;
    }

    /**
     * Wait for containers to reach a specific count
     */
    async waitForContainerCount(functionName, targetCount, timeoutMs = 10000) {
        const startTime = Date.now();
        let currentCount = this.getFunctionContainers(functionName);

        while (currentCount !== targetCount && Date.now() - startTime < timeoutMs) {
            await new Promise(resolve => setTimeout(resolve, 1000));
            currentCount = this.getFunctionContainers(functionName);
        }

        if (currentCount !== targetCount) {
            const elapsed = Date.now() - startTime;
            console.warn(`⚠️ Container count for ${functionName} did not reach ${targetCount} after ${elapsed}ms (current: ${currentCount})`);
        }

        return currentCount;
    }

    /**
     * Get container monitoring snapshot
     */
    getContainerSnapshot() {
        const snapshot = {
            timestamp: new Date().toISOString(),
            totalLambdaContainers: 0,
            functionContainers: {},
            allContainers: []
        };

        try {
            const containers = this.getLambdaContainers();
            snapshot.allContainers = containers;
            snapshot.totalLambdaContainers = containers.length;

            // Count containers per function
            for (const functionName of this.functions) {
                snapshot.functionContainers[functionName] = this.getFunctionContainers(functionName);
            }
        } catch (error) {
            console.warn(`⚠️ Failed to get container snapshot: ${error.message}`);
        }

        return snapshot;
    }

    /**
     * Delete a single function with retries
     */
    async deleteFunction(functionName, retries = this.cleanupRetries) {
        for (let attempt = 1; attempt <= retries; attempt++) {
            try {
                if (this.verbose) {
                    console.log(`🗑️ Deleting function ${functionName} (attempt ${attempt}/${retries})...`);
                }

                await this.client.deleteFunction(functionName);
                this.unregisterFunction(functionName);

                if (this.verbose) {
                    console.log(`✅ Deleted function: ${functionName}`);
                }

                return true;
            } catch (error) {
                if (attempt === retries) {
                    console.error(`❌ Failed to delete function ${functionName} after ${retries} attempts: ${error.message}`);
                    return false;
                } else {
                    if (this.verbose) {
                        console.warn(`⚠️ Delete attempt ${attempt} failed for ${functionName}, retrying...`);
                    }
                    await new Promise(resolve => setTimeout(resolve, this.cleanupDelayMs));
                }
            }
        }
        return false;
    }

    /**
     * Verify all containers are cleaned up for a function
     */
    async verifyContainersCleanedUp(functionName, timeoutMs = 30000) {
        const startTime = Date.now();
        let containerCount = this.getFunctionContainers(functionName);

        while (containerCount > 0 && Date.now() - startTime < timeoutMs) {
            await new Promise(resolve => setTimeout(resolve, 2000));
            containerCount = this.getFunctionContainers(functionName);
        }

        if (containerCount > 0) {
            console.warn(`⚠️ Function ${functionName} still has ${containerCount} container(s) after ${timeoutMs}ms`);
            return false;
        }

        if (this.verbose) {
            console.log(`✅ All containers cleaned up for ${functionName}`);
        }
        return true;
    }

    /**
     * Force remove any remaining containers for a function (both running and exited)
     */
    async forceRemoveContainers(functionName) {
        try {
            // Use -a flag to include stopped/exited containers
            const containers = execSync(
                `docker ps -a --format '{{.Names}}' | grep -E "^lambda-${functionName}-"`,
                { encoding: 'utf8' }
            ).trim().split('\n').filter(Boolean);

            if (containers.length > 0) {
                console.log(`🔨 Force removing ${containers.length} container(s) for ${functionName}...`);

                for (const container of containers) {
                    try {
                        execSync(`docker rm -f ${container}`, { stdio: 'pipe' });
                        if (this.verbose) {
                            console.log(`✅ Force removed container: ${container}`);
                        }
                    } catch (error) {
                        console.warn(`⚠️ Failed to force remove container ${container}: ${error.message}`);
                    }
                }
            }
        } catch (error) {
            // No containers found or command failed - that's okay
            if (this.verbose && error.message && !error.message.includes('Command failed')) {
                console.log(`No containers to force remove for ${functionName}`);
            }
        }
    }

    /**
     * Clean up all registered functions
     */
    async cleanup(options = {}) {
        const forceRemoveContainers = options.forceRemoveContainers !== false; // true by default
        const verifyCleanup = options.verifyCleanup !== false; // true by default
        const parallel = options.parallel !== false; // true by default

        if (this.functions.size === 0) {
            if (this.verbose) {
                console.log('ℹ️ No functions to clean up');
            }
            return { success: true, deleted: 0, failed: 0 };
        }

        console.log(`🧹 Cleaning up ${this.functions.size} function(s)...`);

        const startSnapshot = this.containerTrackingEnabled ? this.getContainerSnapshot() : null;
        if (this.verbose && startSnapshot) {
            console.log(`📊 Pre-cleanup snapshot: ${startSnapshot.totalLambdaContainers} Lambda containers`);
        }

        const functionNames = Array.from(this.functions);
        let deleted = 0;
        let failed = 0;

        // Delete functions
        if (parallel) {
            const results = await Promise.allSettled(
                functionNames.map(name => this.deleteFunction(name))
            );

            results.forEach((result, index) => {
                if (result.status === 'fulfilled' && result.value) {
                    deleted++;
                } else {
                    failed++;
                    console.error(`❌ Failed to delete ${functionNames[index]}`);
                }
            });
        } else {
            for (const functionName of functionNames) {
                const success = await this.deleteFunction(functionName);
                if (success) {
                    deleted++;
                } else {
                    failed++;
                }
            }
        }

        // Verify containers are cleaned up
        if (verifyCleanup && this.containerTrackingEnabled) {
            console.log('🔍 Verifying containers are cleaned up...');

            for (const functionName of functionNames) {
                const cleanedUp = await this.verifyContainersCleanedUp(functionName, 30000);

                if (!cleanedUp && forceRemoveContainers) {
                    console.log(`🔨 Force removing containers for ${functionName}...`);
                    await this.forceRemoveContainers(functionName);
                }
            }
        }

        const endSnapshot = this.containerTrackingEnabled ? this.getContainerSnapshot() : null;
        if (this.verbose && endSnapshot) {
            console.log(`📊 Post-cleanup snapshot: ${endSnapshot.totalLambdaContainers} Lambda containers`);
        }

        console.log(`✅ Cleanup complete: ${deleted} deleted, ${failed} failed`);

        return {
            success: failed === 0,
            deleted,
            failed,
            startSnapshot,
            endSnapshot
        };
    }

    /**
     * Emergency cleanup - force removes ALL Lambda containers
     * Use only when tests are completely broken and need full reset
     */
    async emergencyCleanup() {
        console.log('🚨 EMERGENCY CLEANUP: Removing ALL Lambda containers...');

        try {
            // Stop and remove all Lambda containers
            execSync(
                'docker ps -a --format "{{.Names}}" | grep "^lambda-" | xargs -r docker rm -f',
                { stdio: 'pipe' }
            );
            console.log('✅ Emergency cleanup complete');
        } catch (error) {
            console.warn(`⚠️ Emergency cleanup encountered errors: ${error.message}`);
        }

        // Clear function registry
        this.functions.clear();
    }

    /**
     * Get cleanup status report
     */
    getCleanupStatus() {
        const snapshot = this.getContainerSnapshot();

        return {
            registeredFunctions: Array.from(this.functions),
            registeredCount: this.functions.size,
            containerSnapshot: snapshot,
            cleanupConfiguration: {
                timeoutMs: this.cleanupTimeoutMs,
                retries: this.cleanupRetries,
                delayMs: this.cleanupDelayMs,
                containerTracking: this.containerTrackingEnabled
            }
        };
    }
}

module.exports = CleanupManager;
