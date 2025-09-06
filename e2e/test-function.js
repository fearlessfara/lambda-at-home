/**
 * Unified Lambda Test Function
 * 
 * This function is used by all test scripts to provide consistent testing behavior.
 * It accepts input parameters and a wait parameter to simulate different processing scenarios.
 */

exports.handler = async (event, context) => {
    console.log('Unified test function invoked');
    console.log('Event:', JSON.stringify(event, null, 2));

    // Extract parameters from event
    const waitMs = event.wait || 0;
    const testId = event.testId || 'default';
    const message = event.message || 'Hello from unified test function';
    
    // Simulate processing time if wait parameter is provided
    if (waitMs > 0) {
        console.log(`Waiting for ${waitMs}ms to simulate processing...`);
        await new Promise(resolve => setTimeout(resolve, waitMs));
        console.log('Wait completed');
    }

    // Prepare response
    const response = {
        success: true,
        testId: testId,
        message: message,
        waitMs: waitMs,
        timestamp: new Date().toISOString(),
        nodeVersion: process.version,
        runtime: 'node',
        event: event
    };

    console.log('Response:', JSON.stringify(response, null, 2));
    
    return response;
};
