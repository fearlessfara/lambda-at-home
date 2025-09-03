exports.handler = async (event, context) => {
    console.log('Event:', JSON.stringify(event, null, 2));
    console.log('Context:', JSON.stringify(context, null, 2));
    
    return {
        statusCode: 200,
        body: JSON.stringify({
            message: 'Hello from Lambda@Home!',
            event: event,
            timestamp: new Date().toISOString()
        })
    };
};
