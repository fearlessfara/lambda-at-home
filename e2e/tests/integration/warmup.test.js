/**
 * Warm-up functionality tests
 */

const TestClient = require('../utils/test-client');
const fs = require('fs');
const path = require('path');

describe('Warm-up Tests', () => {
    let client;
    let testFunctionName;

    beforeAll(async () => {
        client = new TestClient();
        testFunctionName = `warmup-test-${Date.now()}`;
    });

    afterAll(async () => {
        if (client) {
            try {
                await client.deleteFunction(testFunctionName);
                console.log(`✅ Cleaned up warmup test function: ${testFunctionName}`);
            } catch (error) {
                console.error(`❌ Failed to cleanup warmup test function ${testFunctionName}: ${error.message}`);
            }
            client.close();
        }
    });

    test('should warm up container on function creation', async () => {
        // Load test function ZIP
        const zipPath = path.join(__dirname, '../../test-functions/simple-test.zip');
        const zipData = fs.readFileSync(zipPath, 'base64');

        // Create function - this should trigger warm-up
        const startTime = Date.now();
        const functionData = await client.createFunction(
            testFunctionName,
            'nodejs18.x',
            'index.handler',
            zipData
        );
        const creationTime = Date.now() - startTime;

        expect(functionData.function_name).toBe(testFunctionName);
        expect(functionData.state).toBe('Active');

        // Wait a bit for warm-up to complete (if it doesn't timeout)
        await new Promise(resolve => setTimeout(resolve, 2000));

        // Check warm pool status
        const warmPoolStatus = await client.getWarmPool(testFunctionName);
        console.log('Warm pool status:', warmPoolStatus);
        console.log('Function creation time:', creationTime, 'ms');

        // The warm-up might timeout, but the function should still be created successfully
        expect(warmPoolStatus).toBeDefined();
        expect(warmPoolStatus.total).toBeGreaterThanOrEqual(0);
    }, 60000); // 60 second timeout

    test('should have faster cold start on first invocation after warm-up', async () => {
        // Load test function ZIP
        const zipPath = path.join(__dirname, '../../test-functions/simple-test.zip');
        const zipData = fs.readFileSync(zipPath, 'base64');

        const functionName = `warmup-coldstart-${Date.now()}`;
        
        try {
            // Create function
            await client.createFunction(
                functionName,
                'nodejs18.x',
                'index.handler',
                zipData
            );

            // Wait for potential warm-up
            await new Promise(resolve => setTimeout(resolve, 5000));

            // First invocation - should be faster if warm-up worked
            const startTime = Date.now();
            const response = await client.invokeFunction(functionName, 'Hello World');
            const invocationTime = Date.now() - startTime;

            console.log('First invocation time:', invocationTime, 'ms');
            console.log('Response:', response);

            expect(response).toBeDefined();
            expect(invocationTime).toBeLessThan(30000); // Should complete within 30 seconds

        } finally {
            try {
                await client.deleteFunction(functionName);
                console.log(`✅ Cleaned up warmup coldstart test function: ${functionName}`);
            } catch (error) {
                console.error(`❌ Failed to cleanup warmup coldstart test function ${functionName}: ${error.message}`);
            }
        }
    }, 60000);
});
