#!/usr/bin/env python3

import json
import os
import sys
import time
import traceback
import asyncio
import websockets
import signal
from typing import Optional, Dict, Any

RUNTIME_API = os.environ.get('AWS_LAMBDA_RUNTIME_API', 'localhost:9001')
FUNCTION_NAME = os.environ.get('AWS_LAMBDA_FUNCTION_NAME')
FUNCTION_VERSION = os.environ.get('AWS_LAMBDA_FUNCTION_VERSION', '1')
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

class WebSocketRuntime:
    def __init__(self):
        self.websocket: Optional[websockets.WebSocketServerProtocol] = None
        self.reconnect_attempts = 0
        self.max_reconnect_attempts = 10
        self.reconnect_delay = 1.0
        self.is_connected = False
        self.should_stop = False

    async def connect(self):
        """Connect to the WebSocket runtime API"""
        ws_url = f'ws://{RUNTIME_API}/2018-06-01/runtime/websocket?fn={FUNCTION_NAME}'
        print(f'Connecting to WebSocket: {ws_url}')

        try:
            self.websocket = await websockets.connect(ws_url)
            self.is_connected = True
            self.reconnect_attempts = 0
            print('WebSocket connected')
            await self.register()
            await self.message_loop()
        except Exception as e:
            print(f'WebSocket connection failed: {e}', file=sys.stderr)
            self.is_connected = False
            await self.handle_reconnect()

    async def register(self):
        """Register with the runtime API"""
        message = {
            'type': 'register',
            'function_name': FUNCTION_NAME,
            'runtime': 'python311',
            'version': FUNCTION_VERSION,
            'instance_id': INSTANCE_ID
        }
        await self.send(message)

    async def message_loop(self):
        """Main message processing loop"""
        try:
            async for message in self.websocket:
                try:
                    data = json.loads(message)
                    await self.handle_message(data)
                except json.JSONDecodeError as e:
                    print(f'Failed to parse WebSocket message: {e}', file=sys.stderr)
                except Exception as e:
                    print(f'Error handling message: {e}', file=sys.stderr)
        except websockets.exceptions.ConnectionClosed:
            print('WebSocket connection closed')
            self.is_connected = False
            await self.handle_reconnect()
        except Exception as e:
            print(f'WebSocket error: {e}', file=sys.stderr)
            self.is_connected = False
            await self.handle_reconnect()

    async def handle_message(self, message: Dict[str, Any]):
        """Handle incoming WebSocket messages"""
        message_type = message.get('type')
        
        if message_type == 'invocation':
            await self.handle_invocation(message)
        elif message_type == 'ping':
            await self.send({'type': 'pong'})
        elif message_type == 'error_response':
            print(f'Server error: {message.get("message", "Unknown error")}', file=sys.stderr)
        else:
            print(f'Unknown message type: {message_type}', file=sys.stderr)

    async def handle_invocation(self, message: Dict[str, Any]):
        """Handle function invocation"""
        request_id = message.get('request_id')
        payload = message.get('payload')
        deadline_ms = message.get('deadline_ms')
        invoked_function_arn = message.get('invoked_function_arn')
        trace_id = message.get('trace_id')
        
        print(f'Received invocation: {request_id}')

        try:
            # Create context object
            context = {
                'function_name': FUNCTION_NAME,
                'function_version': FUNCTION_VERSION,
                'memory_limit_in_mb': MEMORY_SIZE,
                'log_group_name': LOG_GROUP_NAME,
                'log_stream_name': LOG_STREAM_NAME,
                'aws_request_id': request_id,
                'invoked_function_arn': invoked_function_arn,
                'trace_id': trace_id
            }
            
            # Execute the user function
            result = handler(payload, context)
            
            # Send response
            await self.send({
                'type': 'response',
                'request_id': request_id,
                'payload': result,
                'headers': {
                    'X-Amz-Executed-Version': FUNCTION_VERSION
                }
            })
            
            print(f'Response posted for: {request_id}')
            
        except Exception as error:
            print(f'Handler error: {error}', file=sys.stderr)
            
            # Send error
            await self.send({
                'type': 'error',
                'request_id': request_id,
                'error_message': str(error),
                'error_type': type(error).__name__,
                'stack_trace': traceback.format_exc().split('\n'),
                'headers': {
                    'X-Amz-Function-Error': 'Unhandled'
                }
            })

    async def send(self, message: Dict[str, Any]):
        """Send a message via WebSocket"""
        if self.websocket and self.is_connected:
            try:
                await self.websocket.send(json.dumps(message))
            except Exception as e:
                print(f'Failed to send WebSocket message: {e}', file=sys.stderr)
        else:
            print('WebSocket not connected, cannot send message', file=sys.stderr)

    async def handle_reconnect(self):
        """Handle reconnection logic"""
        if self.reconnect_attempts < self.max_reconnect_attempts and not self.should_stop:
            self.reconnect_attempts += 1
            print(f'Attempting to reconnect ({self.reconnect_attempts}/{self.max_reconnect_attempts}) in {self.reconnect_delay}s')
            
            await asyncio.sleep(self.reconnect_delay)
            await self.connect()
            
            # Exponential backoff
            self.reconnect_delay = min(self.reconnect_delay * 2, 30.0)
        else:
            print('Max reconnection attempts reached, falling back to HTTP', file=sys.stderr)
            await self.fallback_to_http()

    async def fallback_to_http(self):
        """Fallback to HTTP runtime"""
        print('Falling back to HTTP runtime...')
        # Import and start the HTTP runtime
        import bootstrap
        # The HTTP runtime will start automatically when imported

    async def shutdown(self):
        """Graceful shutdown"""
        self.should_stop = True
        if self.websocket:
            await self.websocket.close()
        print('WebSocket runtime shutdown complete')

async def main():
    """Main entry point"""
    print('Lambda runtime started with WebSocket support')
    
    runtime = WebSocketRuntime()
    
    # Set up signal handlers for graceful shutdown
    def signal_handler(signum, frame):
        print(f'Received signal {signum}, shutting down...')
        asyncio.create_task(runtime.shutdown())
    
    signal.signal(signal.SIGTERM, signal_handler)
    signal.signal(signal.SIGINT, signal_handler)
    
    try:
        await runtime.connect()
    except KeyboardInterrupt:
        print('Received keyboard interrupt, shutting down...')
        await runtime.shutdown()
    except Exception as e:
        print(f'Runtime error: {e}', file=sys.stderr)
        await runtime.shutdown()

if __name__ == '__main__':
    asyncio.run(main())
