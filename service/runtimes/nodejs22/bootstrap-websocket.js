#!/usr/bin/env node

const WebSocket = require('ws');
const fs = require('fs');
const path = require('path');

// Runtime API configuration (supports values with or without scheme)
const RAW_RUNTIME_API = process.env.AWS_LAMBDA_RUNTIME_API || 'host.docker.internal:9001';
function parseRuntimeApiHostPort(raw) {
  try {
    const url = raw.includes('://') ? new URL(raw) : new URL(`http://${raw}`);
    const hostname = url.hostname;
    const port = url.port ? parseInt(url.port, 10) : (url.protocol === 'https:' ? 443 : 80);
    return { hostname, port };
  } catch (e) {
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

console.log('Lambda Runtime starting with WebSocket...');
console.log('Function:', FUNCTION_NAME);
console.log('Handler:', HANDLER);
console.log('Task Root:', TASK_ROOT);

// Load the user function
let userHandler;
try {
  const handlerPath = path.join(TASK_ROOT, HANDLER.split('.')[0] + '.js');
  const handlerName = HANDLER.split('.')[1] || 'handler';
  delete require.cache[require.resolve(handlerPath)];
  const userModule = require(handlerPath);
  userHandler = userModule[handlerName];
  if (typeof userHandler !== 'function') {
    throw new Error(`Handler '${handlerName}' is not a function in ${handlerPath}`);
  }
} catch (error) {
  console.error('Failed to load handler:', error);
  process.exit(1);
}

let ws = null;
let reconnectTimeout = null;
const RECONNECT_DELAY = 1000;
const MAX_RECONNECT_DELAY = 30000;
let currentReconnectDelay = RECONNECT_DELAY;

function connectWebSocket() {
  return new Promise((resolve, reject) => {
    const wsUrl = `ws://${RUNTIME_API.hostname}:${RUNTIME_API.port}/2018-06-01/runtime/websocket?fn=${encodeURIComponent(FUNCTION_NAME)}&ver=${encodeURIComponent(FUNCTION_VERSION)}`;
    console.log('Connecting to WebSocket:', wsUrl);
    
    ws = new WebSocket(wsUrl, {
      headers: INSTANCE_ID ? { 'X-LambdaH-Instance-Id': INSTANCE_ID } : {}
    });

    ws.on('open', () => {
      console.log('WebSocket connected');
      currentReconnectDelay = RECONNECT_DELAY; // Reset reconnect delay on successful connection
      resolve();
    });

    ws.on('message', async (data) => {
      try {
        const message = JSON.parse(data.toString());
        console.log('Received WebSocket message:', message.type);

        if (message.type === 'invocation') {
          const { request_id, event, deadline_ms } = message;
          console.log('Got invocation:', request_id);

          let result;
          try {
            result = await userHandler(event);
            console.log('Function result:', result);
            
            // Send success response
            const response = {
              type: 'response',
              request_id,
              payload: result
            };
            ws.send(JSON.stringify(response));
            console.log('Posted response for:', request_id);
          } catch (error) {
            console.error('Function error:', error);
            
            // Send error response
            const errorResponse = {
              type: 'error',
              request_id,
              error: {
                errorMessage: error.message,
                errorType: 'Unhandled',
                stackTrace: error.stack
              }
            };
            ws.send(JSON.stringify(errorResponse));
          }
        } else if (message.type === 'ping') {
          // Respond to ping with pong
          ws.send(JSON.stringify({ type: 'pong' }));
        }
      } catch (error) {
        console.error('Error processing WebSocket message:', error);
      }
    });

    ws.on('close', (code, reason) => {
      console.log(`WebSocket closed: ${code} ${reason}`);
      ws = null;
      scheduleReconnect();
    });

    ws.on('error', (error) => {
      console.error('WebSocket error:', error);
      reject(error);
    });
  });
}

function scheduleReconnect() {
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
  }
  
  console.log(`Scheduling reconnect in ${currentReconnectDelay}ms`);
  reconnectTimeout = setTimeout(async () => {
    try {
      await connectWebSocket();
    } catch (error) {
      console.error('Reconnect failed:', error);
      // Exponential backoff with jitter
      currentReconnectDelay = Math.min(currentReconnectDelay * 2, MAX_RECONNECT_DELAY);
      scheduleReconnect();
    }
  }, currentReconnectDelay);
}

// Start WebSocket connection
connectWebSocket().catch((error) => {
  console.error('Failed to connect WebSocket:', error);
  process.exit(1);
});

// Graceful shutdown
process.on('SIGTERM', () => {
  console.log('Received SIGTERM, closing WebSocket...');
  if (ws) {
    ws.close();
  }
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
  }
  process.exit(0);
});

process.on('SIGINT', () => {
  console.log('Received SIGINT, closing WebSocket...');
  if (ws) {
    ws.close();
  }
  if (reconnectTimeout) {
    clearTimeout(reconnectTimeout);
  }
  process.exit(0);
});
