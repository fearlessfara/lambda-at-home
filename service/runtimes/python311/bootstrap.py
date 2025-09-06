#!/usr/bin/env python3

import json
import os
import sys
import urllib.request
import urllib.error
import time
import traceback

RUNTIME_API = os.environ.get('AWS_LAMBDA_RUNTIME_API', 'localhost:9001')
FUNCTION_NAME = os.environ.get('AWS_LAMBDA_FUNCTION_NAME')
FUNCTION_VERSION = os.environ.get('AWS_LAMBDA_FUNCTION_VERSION')
MEMORY_SIZE = os.environ.get('AWS_LAMBDA_FUNCTION_MEMORY_SIZE')
LOG_GROUP_NAME = os.environ.get('AWS_LAMBDA_LOG_GROUP_NAME')
LOG_STREAM_NAME = os.environ.get('AWS_LAMBDA_LOG_STREAM_NAME')
INSTANCE_ID = os.environ.get('LAMBDAH_INSTANCE_ID')

# Load the user's handler
try:
    import lambda_function
    handler = lambda_function.handler
    if not callable(handler):
        raise Exception('Handler function not found in lambda_function.py')
except Exception as e:
    print(f'Failed to load handler: {e}', file=sys.stderr)
    sys.exit(1)

def get_next_invocation():
    """Get the next invocation from the runtime API"""
    url = f'http://{RUNTIME_API}/2018-06-01/runtime/invocation/next?fn={FUNCTION_NAME}'
    
    try:
        req = urllib.request.Request(url)
        if INSTANCE_ID:
            req.add_header('X-LambdaH-Instance-Id', INSTANCE_ID)
        req.add_header('User-Agent', 'lambda-runtime-interface-client')
        
        with urllib.request.urlopen(req) as response:
            data = response.read().decode('utf-8')
            headers = dict(response.headers)
            
            return {
                'requestId': headers.get('lambda-runtime-aws-request-id'),
                'deadline': headers.get('lambda-runtime-deadline-ms'),
                'invokedFunctionArn': headers.get('lambda-runtime-invoked-function-arn'),
                'traceId': headers.get('lambda-runtime-trace-id'),
                'payload': json.loads(data)
            }
    except Exception as e:
        raise Exception(f'Failed to get next invocation: {e}')

def post_response(request_id, response):
    """Post the response back to the runtime API"""
    url = f'http://{RUNTIME_API}/2018-06-01/runtime/invocation/{request_id}/response'
    
    try:
        data = json.dumps(response).encode('utf-8')
        req = urllib.request.Request(url, data=data)
        if INSTANCE_ID:
            req.add_header('X-LambdaH-Instance-Id', INSTANCE_ID)
        req.add_header('Content-Type', 'application/json')
        
        with urllib.request.urlopen(req) as response:
            pass  # Response posted successfully
    except Exception as e:
        raise Exception(f'Failed to post response: {e}')

def post_error(request_id, error):
    """Post an error back to the runtime API"""
    url = f'http://{RUNTIME_API}/2018-06-01/runtime/invocation/{request_id}/error'
    
    try:
        error_data = {
            'errorType': type(error).__name__,
            'errorMessage': str(error),
            'stackTrace': traceback.format_exc().split('\n')
        }
        
        data = json.dumps(error_data).encode('utf-8')
        req = urllib.request.Request(url, data=data)
        if INSTANCE_ID:
            req.add_header('X-LambdaH-Instance-Id', INSTANCE_ID)
        req.add_header('Content-Type', 'application/json')
        
        with urllib.request.urlopen(req) as response:
            pass  # Error posted successfully
    except Exception as e:
        print(f'Failed to post error: {e}', file=sys.stderr)

def main():
    print('Lambda runtime started')
    
    while True:
        try:
            invocation = get_next_invocation()
            print(f'Received invocation: {invocation["requestId"]}')
            
            try:
                context = {
                    'function_name': FUNCTION_NAME,
                    'function_version': FUNCTION_VERSION,
                    'memory_limit_in_mb': MEMORY_SIZE,
                    'log_group_name': LOG_GROUP_NAME,
                    'log_stream_name': LOG_STREAM_NAME,
                    'aws_request_id': invocation['requestId'],
                    'invoked_function_arn': invocation['invokedFunctionArn'],
                    'trace_id': invocation['traceId']
                }
                
                result = handler(invocation['payload'], context)
                post_response(invocation['requestId'], result)
                print(f'Response posted for: {invocation["requestId"]}')
                
            except Exception as error:
                print(f'Handler error: {error}', file=sys.stderr)
                post_error(invocation['requestId'], error)
                
        except Exception as error:
            print(f'Runtime error: {error}', file=sys.stderr)
            # Wait a bit before retrying
            time.sleep(1)

if __name__ == '__main__':
    main()
