#!/usr/bin/env node

const http = require('http');
const fs = require('fs');
const path = require('path');

// Runtime API configuration
const RUNTIME_API = process.env.AWS_LAMBDA_RUNTIME_API || 'host.docker.internal:9001';
const FUNCTION_NAME = process.env.AWS_LAMBDA_FUNCTION_NAME;
const FUNCTION_VERSION = process.env.AWS_LAMBDA_FUNCTION_VERSION || '1';
const HANDLER = process.env.AWS_LAMBDA_FUNCTION_HANDLER || 'index.handler';
const TASK_ROOT = process.env.LAMBDA_TASK_ROOT || '/var/task';

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
function makeRequest(method, path, data = null) {
    return new Promise((resolve, reject) => {
        const options = {
            hostname: RUNTIME_API.split(':')[0],
            port: RUNTIME_API.split(':')[1] || 80,
            path: path,
            method: method,
            headers: {
                'Content-Type': 'application/json',
            }
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
    
    while (true) {
        try {
            // Poll for next invocation
            console.log('Polling for next invocation...');
            const response = await makeRequest('GET', '/2018-06-01/runtime/invocation/next');
            
            if (response.statusCode !== 200) {
                console.error('Failed to get next invocation:', response.statusCode, response.body);
                await new Promise(resolve => setTimeout(resolve, 1000));
                continue;
            }

            const awsRequestId = response.headers['lambda-runtime-aws-request-id'];
            const deadline = response.headers['lambda-runtime-deadline-ms'];
            const invokedFunctionArn = response.headers['lambda-runtime-invoked-function-arn'];
            
            console.log('Got invocation:', awsRequestId);
            
            let payload;
            try {
                payload = JSON.parse(response.body);
            } catch (error) {
                console.error('Failed to parse payload:', error);
                await makeRequest('POST', `/2018-06-01/runtime/invocation/${awsRequestId}/error`, 
                    JSON.stringify({
                        errorMessage: 'Invalid JSON payload',
                        errorType: 'InvalidRequestException'
                    }));
                continue;
            }

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
                    }));
                continue;
            }

            // Post the result
            await makeRequest('POST', `/2018-06-01/runtime/invocation/${awsRequestId}/response`, 
                JSON.stringify(result));
            
            console.log('Posted response for:', awsRequestId);

        } catch (error) {
            console.error('Runtime loop error:', error);
            await new Promise(resolve => setTimeout(resolve, 1000));
        }
    }
}

// Start the runtime
runtimeLoop().catch(console.error);