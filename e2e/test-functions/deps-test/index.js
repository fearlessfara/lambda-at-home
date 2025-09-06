const { v4: uuidv4 } = require('uuid');

exports.handler = async (event) => {
    const { testId, message, input } = event;
    
    console.log('Lambda function with dependencies invoked');
    console.log('Event:', JSON.stringify(event, null, 2));
    
    try {
        // Test UUID generation
        const generatedUuid = uuidv4();
        const isValidUuid = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(generatedUuid);
        
        // Test basic functionality
        const result = {
            success: true,
            testId: testId || 'default',
            message: message || 'Hello from Lambda with dependencies',
            input: input || 'no input provided',
            timestamp: new Date().toISOString(),
            nodeVersion: process.version,
            runtime: 'node',
            uuid: {
                generated: generatedUuid,
                isValid: isValidUuid
            },
            validation: {
                allDependenciesLoaded: true,
                uuidWorking: isValidUuid
            },
            event: event
        };
        
        console.log('Response:', JSON.stringify(result, null, 2));
        
        return result;
        
    } catch (error) {
        console.error('Error in Lambda function:', error);
        
        return {
            success: false,
            error: error.message,
            stack: error.stack,
            testId: testId || 'default'
        };
    }
};
