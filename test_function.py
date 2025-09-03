def handler(event, context):
    print(f'Event: {event}')
    print(f'Context: {context}')
    
    return {
        'statusCode': 200,
        'body': {
            'message': 'Hello from Lambda@Home Python!',
            'event': event,
            'timestamp': context.get('timestamp', 'unknown')
        }
    }
