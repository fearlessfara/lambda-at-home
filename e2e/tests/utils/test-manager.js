/**
 * Test Manager - Manages test functions and cleanup
 */

const fs = require('fs');
const path = require('path');
const TestClient = require('./test-client');
const DockerUtils = require('./docker-utils');

class TestManager {
    constructor() {
        this.client = new TestClient();
        this.functions = new Set();
        this.testFunctionZip = null;
    }

    async setup() {
        // Load test function zip
        const zipPath = path.join(__dirname, '../../test-function.zip');
        if (!fs.existsSync(zipPath)) {
            throw new Error(`Test function zip not found: ${zipPath}`);
        }
        this.testFunctionZip = fs.readFileSync(zipPath).toString('base64');

        // Check server health
        const health = await this.client.healthCheck();
        if (!health.healthy) {
            throw new Error(`Lambda@Home server is not healthy: ${health.error || 'Unknown error'}`);
        }
    }

    async teardown() {
        // Clean up functions using the delete API
        const cleanupPromises = Array.from(this.functions).map(async (functionName) => {
            try {
                const result = await this.client.deleteFunction(functionName);
                console.log(`✅ Deleted function: ${functionName}`);
            } catch (error) {
                console.error(`❌ Failed to delete function ${functionName}: ${error.message}`);
            }
        });

        await Promise.all(cleanupPromises);
        this.functions.clear();
        
        // Note: Containers are automatically cleaned up when functions are deleted
        // No need to manually kill containers - the delete API handles this
        
        // Close HTTP connections
        this.client.close();
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
}

module.exports = TestManager;
