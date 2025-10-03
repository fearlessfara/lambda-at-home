/**
 * Test Manager - Manages test functions and cleanup
 */

const fs = require('fs');
const path = require('path');
const { spawn, execSync } = require('child_process');
const TestClient = require('./test-client');
const DockerUtils = require('./docker-utils');
const CleanupManager = require('./cleanup-manager');

class TestManager {
    constructor(options = {}) {
        this.client = new TestClient();
        this.functions = new Set();
        this.testFunctionZip = null;
        this.cleanupManager = new CleanupManager(this.client, {
            verbose: options.verbose || false,
            cleanupTimeoutMs: options.cleanupTimeoutMs || 60000,
            containerTracking: options.containerTracking !== false
        });
        this.serverProcess = null;
        this.managedServer = options.managedServer !== false;
    }

    async setup() {
        // Build and start the service if managed
        if (this.managedServer && !process.env.SKIP_SERVICE_START) {
            await this.buildAndStartService();
        }

        // Load test function zip
        const zipPath = path.join(__dirname, '../../test-function.zip');
        if (!fs.existsSync(zipPath)) {
            throw new Error(`Test function zip not found: ${zipPath}`);
        }
        this.testFunctionZip = fs.readFileSync(zipPath).toString('base64');

        // Check server health
        await this.waitForServerHealth();
    }

    async buildAndStartService() {
        console.log('🔨 Building Lambda@Home service (release mode)...');
        const projectRoot = path.join(__dirname, '../../../');

        try {
            // Build release version
            execSync('cargo build --release', {
                cwd: projectRoot,
                stdio: 'inherit'
            });
            console.log('✅ Service built successfully');

            // Start the service
            console.log('🚀 Starting Lambda@Home service...');
            const binaryPath = path.join(projectRoot, 'target/release/lambda-at-home-server');

            this.serverProcess = spawn(binaryPath, [], {
                cwd: projectRoot,
                stdio: ['ignore', 'pipe', 'pipe'],
                detached: false
            });

            // Capture server output
            this.serverProcess.stdout.on('data', (data) => {
                if (process.env.VERBOSE_SERVER) {
                    console.log(`[SERVER] ${data.toString().trim()}`);
                }
            });

            this.serverProcess.stderr.on('data', (data) => {
                if (process.env.VERBOSE_SERVER) {
                    console.error(`[SERVER ERROR] ${data.toString().trim()}`);
                }
            });

            this.serverProcess.on('error', (error) => {
                console.error('❌ Server process error:', error);
            });

            this.serverProcess.on('exit', (code, signal) => {
                if (code !== 0 && code !== null) {
                    console.error(`❌ Server exited with code ${code}`);
                }
            });

            // Wait for server to be ready
            await this.waitForServerHealth(30000);
            console.log('✅ Service started successfully');
        } catch (error) {
            throw new Error(`Failed to build/start service: ${error.message}`);
        }
    }

    async waitForServerHealth(maxWaitMs = 10000) {
        const startTime = Date.now();
        let lastError = null;

        while (Date.now() - startTime < maxWaitMs) {
            try {
                const health = await this.client.healthCheck();
                if (health.healthy) {
                    return;
                }
                lastError = health.error || 'Server not healthy';
            } catch (error) {
                lastError = error.message;
            }

            await new Promise(resolve => setTimeout(resolve, 1000));
        }

        throw new Error(`Server did not become healthy within ${maxWaitMs}ms. Last error: ${lastError}`);
    }

    async teardown(options = {}) {
        // Use centralized cleanup manager for proper cleanup with retry and verification
        const cleanupResult = await this.cleanupManager.cleanup({
            parallel: options.parallel !== false,
            verifyCleanup: options.verifyCleanup !== false,
            forceRemoveContainers: options.forceRemoveContainers !== false
        });

        // Clear local function registry
        this.functions.clear();

        // Close HTTP connections
        this.client.close();

        // Stop the managed server if we started it
        if (this.serverProcess && this.managedServer) {
            await this.stopService();
        }

        return cleanupResult;
    }

    async stopService() {
        if (!this.serverProcess) {
            return;
        }

        console.log('🛑 Stopping Lambda@Home service...');

        return new Promise((resolve) => {
            const timeout = setTimeout(() => {
                console.log('⚠️ Service did not stop gracefully, forcing...');
                this.serverProcess.kill('SIGKILL');
                resolve();
            }, 5000);

            this.serverProcess.on('exit', () => {
                clearTimeout(timeout);
                console.log('✅ Service stopped');
                this.serverProcess = null;
                resolve();
            });

            // Send SIGTERM for graceful shutdown
            this.serverProcess.kill('SIGTERM');
        });
    }

    async createTestFunction(name, runtime = 'nodejs22.x') {
        const functionName = `${name}-${Date.now()}`;

        try {
            const functionData = await this.client.createFunction(
                functionName,
                runtime,
                'index.handler',
                this.testFunctionZip
            );

            this.functions.add(functionName);
            this.cleanupManager.registerFunction(functionName);

            // Wait for function to be ready
            await this.waitForFunctionReady(functionName);

            return {
                name: functionName,
                data: functionData,
                runtime: runtime
            };
        } catch (error) {
            throw new Error(`Failed to create test function ${functionName}: ${error.message}`);
        }
    }

    async waitForFunctionReady(functionName, maxWaitMs = 10000) {
        const startTime = Date.now();
        
        while (Date.now() - startTime < maxWaitMs) {
            try {
                const functionData = await this.client.getFunction(functionName);
                if (functionData.state === 'Active') {
                    return functionData;
                }
            } catch (error) {
                // Function not ready yet, continue waiting
            }
            
            await new Promise(resolve => setTimeout(resolve, 500));
        }
        
        throw new Error(`Function ${functionName} did not become ready within ${maxWaitMs}ms`);
    }

    async invokeTestFunction(functionName, testId, message, waitMs = 0, additionalData = {}) {
        const payload = {
            testId: testId,
            message: message,
            wait: waitMs,
            timestamp: new Date().toISOString(),
            ...additionalData
        };

        return await this.client.invokeFunction(functionName, payload);
    }

    async measureInvocation(functionName, payload) {
        const startTime = Date.now();
        const result = await this.client.invokeFunction(functionName, payload);
        const endTime = Date.now();
        
        return {
            result: result,
            duration: endTime - startTime,
            startTime: startTime,
            endTime: endTime
        };
    }

    async runConcurrentInvocations(functionName, count, payloadGenerator) {
        const promises = [];
        
        for (let i = 0; i < count; i++) {
            const payload = payloadGenerator(i);
            promises.push(
                this.measureInvocation(functionName, payload)
                    .catch(error => ({ error: error.message, index: i }))
            );
        }
        
        const results = await Promise.all(promises);
        return results;
    }

    async runSequentialInvocations(functionName, count, payloadGenerator, delayMs = 100) {
        const results = [];
        
        for (let i = 0; i < count; i++) {
            const payload = payloadGenerator(i);
            const result = await this.measureInvocation(functionName, payload);
            results.push(result);
            
            if (delayMs > 0) {
                await new Promise(resolve => setTimeout(resolve, delayMs));
            }
        }
        
        return results;
    }

    generateTestPayload(testId, message, waitMs = 0, additionalData = {}) {
        return {
            testId: testId,
            message: message,
            wait: waitMs,
            timestamp: new Date().toISOString(),
            ...additionalData
        };
    }

    generateConcurrentPayload(index, baseTestId, message, waitMs = 0) {
        return this.generateTestPayload(
            `${baseTestId}-${index}`,
            `${message} ${index}`,
            waitMs,
            { index: index }
        );
    }

    generateSequentialPayload(index, baseTestId, message, waitMs = 0) {
        return this.generateTestPayload(
            `${baseTestId}-seq-${index}`,
            `${message} ${index}`,
            waitMs,
            { index: index, sequence: true }
        );
    }

    // Container monitoring helpers - delegate to CleanupManager

    /**
     * Get current count of containers for a function
     */
    getFunctionContainerCount(functionName) {
        return this.cleanupManager.getFunctionContainers(functionName);
    }

    /**
     * Wait for containers to reach a specific count
     */
    async waitForContainerCount(functionName, targetCount, timeoutMs = 10000) {
        return await this.cleanupManager.waitForContainerCount(functionName, targetCount, timeoutMs);
    }

    /**
     * Get a snapshot of all Lambda containers
     */
    getContainerSnapshot() {
        return this.cleanupManager.getContainerSnapshot();
    }

    /**
     * Get cleanup status report
     */
    getCleanupStatus() {
        return this.cleanupManager.getCleanupStatus();
    }

    /**
     * Manually delete a function and unregister it
     */
    async deleteFunction(functionName) {
        try {
            await this.client.deleteFunction(functionName);
            this.functions.delete(functionName);
            this.cleanupManager.unregisterFunction(functionName);
            return true;
        } catch (error) {
            console.error(`Failed to delete function ${functionName}: ${error.message}`);
            return false;
        }
    }

    /**
     * Emergency cleanup - removes ALL Lambda containers
     */
    async emergencyCleanup() {
        await this.cleanupManager.emergencyCleanup();
        this.functions.clear();
    }
}

module.exports = TestManager;
