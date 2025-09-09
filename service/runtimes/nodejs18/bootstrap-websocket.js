#!/usr/bin/env node

const WebSocket = require('ws');
const fs = require('fs');
const path = require('path');

// Runtime API configuration (supports values with or without scheme)
const RAW_RUNTIME_API = process.env.AWS_LAMBDA_RUNTIME_API || 'host.docker.internal:9001';
function parseRuntimeApiHostPort(raw) {
  try {
    // Ensure URL has a scheme for URL parsing
    const url = raw.includes('://') ? new URL(raw) : new URL(`http://${raw}`);
    const hostname = url.hostname;
    const port = url.port ? parseInt(url.port, 10) : (url.protocol === 'https:' ? 443 : 80);
    return { hostname, port };
  } catch (e) {
    // Fallback to naïve split host:port
    const [host, p] = raw.split(':');
    return { hostname: host, port: p ? parseInt(p, 10) : 80 };
  }
}
const RUNTIME_API = parseRuntimeApiHostPort(RAW_RUNTIME_API);
const FUNCTION_NAME = process.env.AWS_LAMBDA_FUNCTION_NAME;
const FUNCTION_VERSION = process.env.AWS_LAMBDA_FUNCTION_VERSION || '1';
const HANDLER = process.env.AWS_LAMBDA_FUNCTION_HANDLER || 'index.handler';
const TASK_ROOT = process.env.LAMBDA_TASK_ROOT || '/var/task';
const INSTANCE_ID = process.env.LAMBDAH_INSTANCE_ID;

console.log('Lambda Runtime starting with WebSocket support...');
console.log('Function:', FUNCTION_NAME);
console.log('Handler:', HANDLER);
console.log('Task Root:', TASK_ROOT);

// Load the user function
let userHandler;
try {
    const handlerPath = path.join(TASK_ROOT, HANDLER.split('.')[0] + '.js');
    console.log('Loading handler from:', handlerPath);
    
    // Load the actual user function
    delete require.cache[require.resolve(handlerPath)];
    const userModule = require(handlerPath);
    const handlerName = HANDLER.split('.')[1] || 'handler';
    userHandler = userModule[handlerName];
    
    if (typeof userHandler !== 'function') {
        throw new Error(`Handler '${handlerName}' is not a function in ${handlerPath}`);
    }
    
    console.log('Successfully loaded handler:', handlerName);
} catch (error) {
    console.error('Failed to load handler:', error);
    process.exit(1);
}

// WebSocket message types
const MessageType = {
    REGISTER: 'register',
    INVOCATION: 'invocation',
    RESPONSE: 'response',
    ERROR: 'error',
    PING: 'ping',
    PONG: 'pong',
    ERROR_RESPONSE: 'error_response'
};

class WebSocketRuntime {
    constructor() {
        this.ws = null;
        this.reconnectAttempts = 0;
        this.maxReconnectAttempts = 10;
        this.reconnectDelay = 1000;
        this.isConnected = false;
    }

    connect() {
        const wsUrl = `ws://${RUNTIME_API.hostname}:${RUNTIME_API.port}/2018-06-01/runtime/websocket?fn=${encodeURIComponent(FUNCTION_NAME)}`;
        console.log('Connecting to WebSocket:', wsUrl);

        this.ws = new WebSocket(wsUrl);

        this.ws.on('open', () => {
            console.log('WebSocket connected');
            this.isConnected = true;
            this.reconnectAttempts = 0;
            this.register();
        });

        this.ws.on('message', (data) => {
            try {
                const message = JSON.parse(data.toString());
                this.handleMessage(message);
            } catch (error) {
                console.error('Failed to parse WebSocket message:', error);
            }
        });

        this.ws.on('close', (code, reason) => {
            console.log(`WebSocket closed: ${code} ${reason}`);
            this.isConnected = false;
            this.handleReconnect();
        });

        this.ws.on('error', (error) => {
            console.error('WebSocket error:', error);
            this.isConnected = false;
        });
    }

    register() {
        const message = {
            type: MessageType.REGISTER,
            function_name: FUNCTION_NAME,
            runtime: 'nodejs18.x',
            version: FUNCTION_VERSION,
            instance_id: INSTANCE_ID
        };
        this.send(message);
    }

    handleMessage(message) {
        switch (message.type) {
            case MessageType.INVOCATION:
                this.handleInvocation(message);
                break;
            case MessageType.PING:
                this.send({ type: MessageType.PONG });
                break;
            case MessageType.ERROR_RESPONSE:
                console.error('Server error:', message.message);
                break;
            default:
                console.warn('Unknown message type:', message.type);
        }
    }

    async handleInvocation(message) {
        const { request_id, payload, deadline_ms, invoked_function_arn, trace_id } = message;
        console.log('Got invocation:', request_id);

        try {
            // Execute the user function
            const result = await userHandler(payload);
            console.log('Function result:', result);

            // Send response
            this.send({
                type: MessageType.RESPONSE,
                request_id: request_id,
                payload: result,
                headers: {
                    'X-Amz-Executed-Version': FUNCTION_VERSION
                }
            });

            console.log('Posted response for:', request_id);
        } catch (error) {
            console.error('Function error:', error);
            
            // Send error
            this.send({
                type: MessageType.ERROR,
                request_id: request_id,
                error_message: error.message,
                error_type: 'Unhandled',
                stack_trace: error.stack ? error.stack.split('\n') : undefined,
                headers: {
                    'X-Amz-Function-Error': 'Unhandled'
                }
            });
        }
    }

    send(message) {
        if (this.ws && this.isConnected) {
            try {
                this.ws.send(JSON.stringify(message));
            } catch (error) {
                console.error('Failed to send WebSocket message:', error);
            }
        } else {
            console.warn('WebSocket not connected, cannot send message');
        }
    }

    handleReconnect() {
        if (this.reconnectAttempts < this.maxReconnectAttempts) {
            this.reconnectAttempts++;
            console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts}) in ${this.reconnectDelay}ms`);
            
            setTimeout(() => {
                this.connect();
            }, this.reconnectDelay);
            
            // Exponential backoff
            this.reconnectDelay = Math.min(this.reconnectDelay * 2, 30000);
        } else {
            console.error('Max reconnection attempts reached, falling back to HTTP');
            this.fallbackToHttp();
        }
    }

    fallbackToHttp() {
        console.log('Falling back to HTTP runtime...');
        // Import and start the HTTP runtime
        const httpRuntime = require('./bootstrap.js');
        // The HTTP runtime will start automatically when imported
    }
}

// Start WebSocket runtime
const runtime = new WebSocketRuntime();
runtime.connect();

// Handle graceful shutdown
process.on('SIGTERM', () => {
    console.log('Received SIGTERM, closing WebSocket connection');
    if (runtime.ws) {
        runtime.ws.close();
    }
    process.exit(0);
});

process.on('SIGINT', () => {
    console.log('Received SIGINT, closing WebSocket connection');
    if (runtime.ws) {
        runtime.ws.close();
    }
    process.exit(0);
});
