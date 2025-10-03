#!/usr/bin/env node

const http = require('http');
const fs = require('fs');
const path = require('path');

// Check if WebSocket should be used
const USE_WEBSOCKET = process.env.LAMBDA_USE_WEBSOCKET !== 'false';
const HAS_WS_LIBRARY = (() => {
  try {
    require.resolve('ws');
    return true;
  } catch {
    return false;
  }
})();

// Use WebSocket if available and not explicitly disabled
if (USE_WEBSOCKET && HAS_WS_LIBRARY) {
  console.log('Using WebSocket runtime (ws library available)');
  require('./bootstrap-websocket.js');
  process.exit(0);
} else if (USE_WEBSOCKET && !HAS_WS_LIBRARY) {
  console.log('WebSocket requested but ws library not available, falling back to HTTP');
} else {
  console.log('Using HTTP runtime (WebSocket disabled)');
}

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

console.log('Lambda Runtime starting...');
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

function makeRequest(method, path, data = null, extraHeaders = {}) {
  return new Promise((resolve, reject) => {
    const options = {
      hostname: RUNTIME_API.hostname,
      port: RUNTIME_API.port,
      path,
      method,
      headers: {
        'Content-Type': 'application/json',
        ...extraHeaders,
      },
    };
    if (data) options.headers['Content-Length'] = Buffer.byteLength(data);
    const req = http.request(options, (res) => {
      let body = '';
      res.on('data', (chunk) => body += chunk);
      res.on('end', () => resolve({ statusCode: res.statusCode, headers: res.headers, body }));
    });
    req.on('error', reject);
    if (data) req.write(data);
    req.end();
  });
}

async function runtimeLoop() {
  const query = new URLSearchParams({ fn: FUNCTION_NAME });
  while (true) {
    try {
      const url = `/2018-06-01/runtime/invocation/next?${query.toString()}`;
      console.log('Waiting for next invocation at', url);
      const response = await makeRequest('GET', url, null, INSTANCE_ID ? { 'X-LambdaH-Instance-Id': INSTANCE_ID } : {});
      console.log('Response status:', response.statusCode);
      if (response.statusCode !== 200) {
        console.error('Failed to get next invocation:', response.statusCode, response.body);
        await new Promise(r => setTimeout(r, 1000));
        continue;
      }
      const awsRequestId = response.headers['lambda-runtime-aws-request-id'];
      const deadlineMs = response.headers['lambda-runtime-deadline-ms'];
      let event;
      try { event = response.body ? JSON.parse(response.body) : undefined; } catch (e) { event = undefined; }
      console.log('Got invocation:', awsRequestId);

      let result;
      try {
        result = await userHandler(event);
        console.log('Function result:', result);
      } catch (error) {
        await makeRequest('POST', `/2018-06-01/runtime/invocation/${awsRequestId}/error`, JSON.stringify({
          errorMessage: error.message,
          errorType: 'Unhandled',
          stackTrace: error.stack,
        }), { 'X-Amz-Function-Error': 'Unhandled', ...(INSTANCE_ID ? { 'X-LambdaH-Instance-Id': INSTANCE_ID } : {}) });
        continue;
      }

      await makeRequest('POST', `/2018-06-01/runtime/invocation/${awsRequestId}/response`, JSON.stringify(result), INSTANCE_ID ? { 'X-LambdaH-Instance-Id': INSTANCE_ID } : {});
      console.log('Posted response for:', awsRequestId);
    } catch (err) {
      console.error('Runtime loop error:', err);
      await new Promise(r => setTimeout(r, 1000));
    }
  }
}

runtimeLoop().catch(console.error);
