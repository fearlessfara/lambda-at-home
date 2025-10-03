import json
import time
from datetime import datetime

def handler(event, context):
    print('WebSocket runtime test function invoked')
    print('Event:', json.dumps(event, indent=2))
    print('Context:', json.dumps({
        'function_name': context.get('function_name'),
        'aws_request_id': context.get('aws_request_id'),
        'memory_limit_in_mb': context.get('memory_limit_in_mb')
    }, indent=2))
    
    return {
        'statusCode': 200,
        'body': json.dumps({
            'message': 'Hello from WebSocket runtime!',
            'timestamp': datetime.now().isoformat(),
            'event': event,
            'runtime': 'python311'
        })
    }
