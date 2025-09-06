#!/usr/bin/env node

const http = require('http');
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

console.log('Lambda Runtime starting...');
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

// Runtime API client
function makeRequest(method, path, data = null, extraHeaders = {}) {
    return new Promise((resolve, reject) => {
        const options = {
            hostname: RUNTIME_API.hostname,
            port: RUNTIME_API.port,
            path: path,
            method: method,
            headers: {
                'Content-Type': 'application/json',
                ...extraHeaders,
            },
        };

        if (data) {
            options.headers['Content-Length'] = Buffer.byteLength(data);
        }

        const req = http.request(options, (res) => {
            let body = '';
            res.on('data', (chunk) => {
                body += chunk;
            });
            res.on('end', () => {
                try {
                    const result = {
                        statusCode: res.statusCode,
                        headers: res.headers,
                        body: body
                    };
                    resolve(result);
                } catch (error) {
                    reject(error);
                }
            });
        });

        req.on('error', (error) => {
            reject(error);
        });

        if (data) {
            req.write(data);
        }
        req.end();
    });
}

// Main runtime loop
async function runtimeLoop() {
    console.log('Starting runtime loop...');
    
    // Build query parameters for runtime API (simplified to just function name)
    const queryParams = new URLSearchParams({
        fn: FUNCTION_NAME
    });
    
    while (true) {
        try {
            // Long-lived GET: this call blocks until work is available
            const url = `/2018-06-01/runtime/invocation/next?${queryParams.toString()}`;
            console.log('Waiting for next invocation at', url);
            const response = await makeRequest('GET', url, null, INSTANCE_ID ? { 'X-LambdaH-Instance-Id': INSTANCE_ID } : {});
            
            console.log('Response status:', response.statusCode);
            // Note: body may be large; avoid logging full payload in production
            
            if (response.statusCode !== 200) {
                console.error('Failed to get next invocation:', response.statusCode, response.body);
                await new Promise(resolve => setTimeout(resolve, 1000));
                continue;
            }

            // Parse AWS-style response: headers carry metadata, body is event JSON
            const awsRequestId = response.headers['lambda-runtime-aws-request-id'];
            const deadlineHeader = response.headers['lambda-runtime-deadline-ms'];
            const deadline = deadlineHeader ? parseInt(deadlineHeader, 10) : undefined;
            let payload;
            try {
                payload = response.body && response.body.length ? JSON.parse(response.body) : undefined;
            } catch (error) {
                console.error('Failed to parse event JSON:', error);
                payload = undefined;
            }
            
            console.log('Got invocation:', awsRequestId);

            // Execute the user function
            let result;
            try {
                result = await userHandler(payload);
                console.log('Function result:', result);
            } catch (error) {
                console.error('Function error:', error);
                await makeRequest('POST', `/2018-06-01/runtime/invocation/${awsRequestId}/error`, 
                    JSON.stringify({
                        errorMessage: error.message,
                        errorType: 'Unhandled',
                        stackTrace: error.stack
                    }),
                    { 'X-Amz-Function-Error': 'Unhandled', ...(INSTANCE_ID ? { 'X-LambdaH-Instance-Id': INSTANCE_ID } : {}) }
                );
                continue;
            }

            // Post the result
            await makeRequest('POST', `/2018-06-01/runtime/invocation/${awsRequestId}/response`, 
                JSON.stringify(result),
                INSTANCE_ID ? { 'X-LambdaH-Instance-Id': INSTANCE_ID } : {}
            );
            
            console.log('Posted response for:', awsRequestId);

        } catch (error) {
            console.error('Runtime loop error:', error);
            await new Promise(resolve => setTimeout(resolve, 1000));
        }
    }
}

// Start the runtime
runtimeLoop().catch(console.error);
