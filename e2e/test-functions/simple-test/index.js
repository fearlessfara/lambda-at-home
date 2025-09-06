exports.handler = async (event) => {
    console.log('Simple Lambda function invoked');
    console.log('Event:', JSON.stringify(event, null, 2));
    
    return {
        statusCode: 200,
        body: JSON.stringify({
            success: true,
            message: 'Hello from simple Lambda function',
            timestamp: new Date().toISOString(),
            nodeVersion: process.version,
            runtime: 'node',
            event: event
        })
    };
};
