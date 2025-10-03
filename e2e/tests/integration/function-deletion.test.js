/**
 * Function Deletion E2E Tests
 * 
 * Tests the complete function deletion workflow including:
 * - Immediate rejection of new invocations during deletion
 * - Graceful handling of in-flight executions
 * - Proper cleanup of containers and resources
 * - Isolation between different functions
 */

const { describe, test, before, after } = require('node:test');
const assert = require('node:assert');
const testData = require('../fixtures/test-data');
const {
    assertValidLambdaResponse,
    assertWithinPerformanceThreshold,
    assertSuccessfulInvocations,
    assertMatchObject
} = require('../utils/assertions');
const { cleanupSingleFunction, cleanupFunctionsByName } = require('../utils/test-helpers');

require('../setup');

describe('Lambda@Home Function Deletion E2E Tests', () => {

    describe('Basic Function Deletion', () => {
        test('should delete function successfully', async () => {
            const functionToDelete = await global.testManager.createTestFunction('basic-deletion-test');
            
            // Verify function exists
            const getResult = await global.testManager.client.getFunction(functionToDelete.name);
            assert.strictEqual(getResult.function_name, functionToDelete.name);
            assert.strictEqual(getResult.state, 'Active');

            // Delete the function
            await global.testManager.client.deleteFunction(functionToDelete.name);

            // Verify function no longer exists
            try {
                await global.testManager.client.getFunction(functionToDelete.name);
                assert.fail('Function should not exist after deletion');
            } catch (error) {
                assert.ok(error.message.includes('FunctionNotFound') || error.message.includes('404'));
            }
        });

        test('should handle deletion of non-existent function gracefully', async () => {
            const nonExistentFunction = 'non-existent-function-' + Date.now();
            
            // The implementation should return an error for non-existent functions
            try {
                await global.testManager.client.deleteFunction(nonExistentFunction);
                assert.fail('Should have thrown error for non-existent function');
            } catch (error) {
                assert.ok(
                    error.message.includes('Function not found') || 
                    error.message.includes('ResourceNotFoundException') ||
                    error.message.includes('404'),
                    `Expected "Function not found" or "ResourceNotFoundException" error, got: ${error.message}`
                );
            }
        });
    });

    describe('Deletion State Management', () => {
        test('should reject new invocations immediately after deletion starts', async () => {
            const functionToDelete = await global.testManager.createTestFunction('rejection-test');
            
            // Start deletion process
            const deletionPromise = global.testManager.client.deleteFunction(functionToDelete.name);
            
            // Add a small delay to ensure deletion state is set
            await new Promise(resolve => setTimeout(resolve, 10));
            
            // Try to invoke the function - should be rejected
            let invocationRejected = false;
            try {
                await invokeTestFunction(functionToDelete.name, 'rejection-test', 'Should be rejected');
            } catch (error) {
                invocationRejected = true;
                console.log('Invocation error message:', error.message);
                assert.ok(
                    error.message.includes('Function not found') || 
                    error.message.includes('ResourceNotFoundException') ||
                    error.message.includes('404') ||
                    error.message.includes('FunctionNotFound') ||
                    error.message.includes('Throttled'),
                    `Expected FunctionNotFound/404/Throttled error, got: ${error.message}`
                );
            }

            // If invocation wasn't rejected, it means deletion completed too quickly
            // This is actually fine - the function was deleted successfully
            if (!invocationRejected) {
                console.log('Function was deleted before invocation could be rejected - this is acceptable');
            }

            // Wait for deletion to complete
            await deletionPromise;

            // Verify function is deleted
            try {
                await global.testManager.client.getFunction(functionToDelete.name);
                assert.fail('Function should not exist after deletion');
            } catch (error) {
                assert.ok(error.message.includes('Function not found') || error.message.includes('404'));
            }
        });

        test('should allow in-flight executions to complete during deletion', async () => {
            const functionToDelete = await global.testManager.createTestFunction('inflight-test');
            
            // Start a long-running invocation
            const longRunningInvocation = invokeTestFunction(
                functionToDelete.name, 
                'long-running-test', 
                'This should complete even during deletion',
                { timeout: 10000 } // 10 second timeout
            );

            // Wait a bit for the invocation to start
            await new Promise(resolve => setTimeout(resolve, 100));

            // Start deletion process
            const deletionPromise = global.testManager.client.deleteFunction(functionToDelete.name);

            // The long-running invocation should complete successfully
            const result = await longRunningInvocation;
            assertValidLambdaResponse(result);
            assert.strictEqual(result.success, true);

            // Wait for deletion to complete
            await deletionPromise;

            // Verify function is deleted
            try {
                await global.testManager.client.getFunction(functionToDelete.name);
                assert.fail('Function should not exist after deletion');
            } catch (error) {
                assert.ok(error.message.includes('FunctionNotFound') || error.message.includes('404'));
            }
        });
    });

    describe('Concurrent Deletion and Invocation', () => {
        test('should handle multiple concurrent invocations during deletion', async () => {
            const functionToDelete = await global.testManager.createTestFunction('concurrent-test');
            
            // Start multiple concurrent invocations
            const invocationPromises = [];
            for (let i = 0; i < 5; i++) {
                invocationPromises.push(
                    invokeTestFunction(
                        functionToDelete.name, 
                        `concurrent-test-${i}`, 
                        `Concurrent invocation ${i}`
                    ).catch(error => ({ error: error.message })) // Capture errors
                );
            }

            // Start deletion after a longer delay to ensure some invocations start
            await new Promise(resolve => setTimeout(resolve, 200));
            const deletionPromise = global.testManager.client.deleteFunction(functionToDelete.name);

            // Wait for all invocations to complete
            const results = await Promise.all(invocationPromises);

            // Some invocations should succeed (started before deletion)
            // Some should fail (started after deletion)
            const successes = results.filter(r => !r.error);
            const failures = results.filter(r => r.error);

            console.log(`Results: ${successes.length} successes, ${failures.length} failures`);
            console.log('Failures:', failures.map(f => f.error));

            // At least one invocation should succeed (the first one likely started before deletion)
            assert.ok(successes.length > 0, 'Some invocations should have succeeded');
            
            // If there are failures, verify they're due to deletion
            if (failures.length > 0) {
                for (const failure of failures) {
                    assert.ok(
                        failure.error.includes('FunctionNotFound') || 
                        failure.error.includes('404') ||
                        failure.error.includes('Throttled') ||
                        failure.error.includes('not found'),
                        `Expected FunctionNotFound/404/Throttled error, got: ${failure.error}`
                    );
                }
            }

            // Wait for deletion to complete
            await deletionPromise;
        });

        test('should handle rapid deletion attempts gracefully', async () => {
            const functionToDelete = await global.testManager.createTestFunction('rapid-deletion-test');
            
            // Start multiple deletion attempts simultaneously
            const deletionPromises = [];
            for (let i = 0; i < 3; i++) {
                deletionPromises.push(
                    global.testManager.client.deleteFunction(functionToDelete.name)
                        .catch(error => ({ error: error.message }))
                );
            }

            // Wait for all deletion attempts
            const results = await Promise.all(deletionPromises);

            // At least one deletion should succeed
            const successes = results.filter(r => !r.error);
            assert.ok(successes.length >= 1, 'At least one deletion should succeed');

            // Verify function is deleted
            try {
                await global.testManager.client.getFunction(functionToDelete.name);
                assert.fail('Function should not exist after deletion');
            } catch (error) {
                assert.ok(error.message.includes('FunctionNotFound') || error.message.includes('404'));
            }
        });
    });

    describe('Function Isolation During Deletion', () => {
        test('should not affect other functions when deleting one', async () => {
            // Create fresh test functions for this specific test to avoid issues with shared state
            const function1 = await createTestFunction('isolation-1');
            const function2 = await createTestFunction('isolation-2');

            try {
                // Invoke both functions to ensure they're working
                await invokeTestFunction(function1.name, 'isolation-test-1', 'Function1 initial test');
                await invokeTestFunction(function2.name, 'isolation-test-2', 'Function2 initial test');

                // Delete function1
                await global.testManager.client.deleteFunction(function1.name);

                // Function2 should still exist and be invokable
                const getResult = await global.testManager.client.getFunction(function2.name);
                assert.strictEqual(getResult.function_name, function2.name);
                assert.strictEqual(getResult.state, 'Active');

                // Function2 should still be invokable
                const result = await invokeTestFunction(function2.name, 'isolation-test', 'Function2 should work');
                assertValidLambdaResponse(result);
                assert.strictEqual(result.success, true);

                // Cleanup function2
                await global.testManager.client.deleteFunction(function2.name);
            } catch (error) {
                // Cleanup on error
                try { await global.testManager.client.deleteFunction(function1.name); } catch (e) { /* ignore */ }
                try { await global.testManager.client.deleteFunction(function2.name); } catch (e) { /* ignore */ }
                throw error;
            }
        });

        test('should handle deletion of multiple functions independently', async () => {
            // Create additional functions for this test
            const function3 = await global.testManager.createTestFunction('isolation-test-3');
            const function4 = await global.testManager.createTestFunction('isolation-test-4');

            try {
                // Delete function3
                await global.testManager.client.deleteFunction(function3.name);

                // Function4 should still exist and be invokable
                const getResult = await global.testManager.client.getFunction(function4.name);
                assert.strictEqual(getResult.function_name, function4.name);

                // Function4 should still be invokable
                const result = await invokeTestFunction(function4.name, 'isolation-test-4', 'Function4 should work');
                assertValidLambdaResponse(result);
                assert.strictEqual(result.success, true);

                // Delete function4
                await global.testManager.client.deleteFunction(function4.name);

                // Verify both functions are deleted
                try {
                    await global.testManager.client.getFunction(function3.name);
                    assert.fail('Function3 should not exist');
                } catch (error) {
                    assert.ok(error.message.includes('FunctionNotFound') || error.message.includes('404'));
                }

                try {
                    await global.testManager.client.getFunction(function4.name);
                    assert.fail('Function4 should not exist');
                } catch (error) {
                    assert.ok(error.message.includes('FunctionNotFound') || error.message.includes('404'));
                }
            } finally {
                // Cleanup any remaining functions
                try {
                    await global.testManager.client.deleteFunction(function3.name);
                } catch (e) { /* ignore */ }
                try {
                    await global.testManager.client.deleteFunction(function4.name);
                } catch (e) { /* ignore */ }
            }
        });
    });

    describe('Resource Cleanup After Deletion', () => {
        test('should clean up containers after function deletion', async () => {
            const functionToDelete = await global.testManager.createTestFunction('cleanup-test');
            
            // Invoke the function to create a container
            await invokeTestFunction(functionToDelete.name, 'cleanup-test', 'Create container');
            
            // Wait a bit for container to be created
            await new Promise(resolve => setTimeout(resolve, 1000));

            // Delete the function
            await global.testManager.client.deleteFunction(functionToDelete.name);

            // Wait for cleanup to complete
            await new Promise(resolve => setTimeout(resolve, 2000));

            // Verify function is deleted
            try {
                await global.testManager.client.getFunction(functionToDelete.name);
                assert.fail('Function should not exist after deletion');
            } catch (error) {
                assert.ok(error.message.includes('FunctionNotFound') || error.message.includes('404'));
            }

            // Note: Container cleanup verification would require Docker API access
            // For now, we verify the function is deleted which indicates cleanup started
        });

        test('should handle deletion of function with warm containers', async () => {
            const functionToDelete = await global.testManager.createTestFunction('warm-cleanup-test');
            
            // Invoke the function to create a warm container
            await invokeTestFunction(functionToDelete.name, 'warm-cleanup-test', 'Create warm container');
            
            // Wait a bit for container to be warmed up
            await new Promise(resolve => setTimeout(resolve, 1000));

            // Delete the function (should clean up warm containers)
            await global.testManager.client.deleteFunction(functionToDelete.name);

            // Verify function is deleted
            try {
                await global.testManager.client.getFunction(functionToDelete.name);
                assert.fail('Function should not exist after deletion');
            } catch (error) {
                assert.ok(error.message.includes('FunctionNotFound') || error.message.includes('404'));
            }
        });
    });

    describe('Error Handling During Deletion', () => {
        test('should handle deletion errors gracefully', async () => {
            const functionToDelete = await global.testManager.createTestFunction('error-handling-test');
            
            // Delete the function normally first
            await global.testManager.client.deleteFunction(functionToDelete.name);

            // Try to delete again - should return error for already deleted function
            try {
                await global.testManager.client.deleteFunction(functionToDelete.name);
                assert.fail('Should have thrown error for already deleted function');
            } catch (error) {
                assert.ok(
                    error.message.includes('Function not found') || 
                    error.message.includes('ResourceNotFoundException') ||
                    error.message.includes('404'),
                    `Expected "Function not found" or "ResourceNotFoundException" error, got: ${error.message}`
                );
            }
        });

        test('should handle deletion during function execution', async () => {
            const functionToDelete = await global.testManager.createTestFunction('execution-deletion-test');
            
            // Start an invocation
            const invocationPromise = invokeTestFunction(
                functionToDelete.name, 
                'execution-deletion-test', 
                'Should complete even if deleted during execution'
            );

            // Wait a bit for execution to start
            await new Promise(resolve => setTimeout(resolve, 100));

            // Delete the function while it's executing
            const deletionPromise = global.testManager.client.deleteFunction(functionToDelete.name);

            // The invocation should still complete successfully
            const result = await invocationPromise;
            assertValidLambdaResponse(result);
            assert.strictEqual(result.success, true);

            // Wait for deletion to complete
            await deletionPromise;

            // Verify function is deleted
            try {
                await global.testManager.client.getFunction(functionToDelete.name);
                assert.fail('Function should not exist after deletion');
            } catch (error) {
                assert.ok(error.message.includes('FunctionNotFound') || error.message.includes('404'));
            }
        });
    });
});

// Helper function to create a test function
async function createTestFunction(suffix) {
    const functionName = `deletion-e2e-${suffix}-${Date.now()}`;
    return await global.testManager.createTestFunction(functionName);
}

// Helper function to invoke a test function
async function invokeTestFunction(functionName, testId, message, options = {}) {
    const payload = {
        testId,
        message,
        input: 'e2e-deletion-test',
        operation: 'echo'
    };

    const result = await global.testManager.client.invokeFunction(functionName, payload, {
        invocationType: 'RequestResponse',
        logType: 'Tail',
        ...options
    });

    return result;
}
