exports.handler = async (event) => {
    console.log('WebSocket runtime test function invoked');
    console.log('Event:', JSON.stringify(event, null, 2));
    
    return {
        statusCode: 200,
        body: JSON.stringify({
            message: 'Hello from WebSocket runtime!',
            timestamp: new Date().toISOString(),
            event: event
        })
    };
};
