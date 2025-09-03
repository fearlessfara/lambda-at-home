const express = require('express');
const path = require('path');
const axios = require('axios');

class LambdaRuntimeInterface {
    constructor() {
        this.app = express();
        this.app.use(express.json());
        this.handler = null;
        this.isShuttingDown = false;
        this.lambdaRuntimeEndpoint = process.env.AWS_LAMBDA_RUNTIME_API || 'http://host.docker.internal:3000';
        this.containerId = process.env.CONTAINER_ID || `container-${Date.now()}`;
        this.functionHandler = process.env.HANDLER || 'index.handler';

        // Handle shutdown signals
        process.on('SIGTERM', () => this.handleShutdown());
        process.on('SIGINT', () => this.handleShutdown());
    }

    async init() {
        try {
            console.log(JSON.stringify({
                timestamp: new Date().toISOString(),
                level: 'INFO',
                message: 'Lambda RIC initialization started',
                handler: this.functionHandler,
                containerId: this.containerId
            }));

            // Parse handler string from environment
            const [module, func] = this.functionHandler.split('.');
            
            // Load user code
            const userCodePath = path.join(process.cwd(), module);
            const userCode = require(userCodePath);
            this.handler = userCode[func];

            if (typeof this.handler !== 'function') {
                throw new Error(`Handler ${this.functionHandler} is not a function`);
            }

            // Initialize health check server
            this.setupHealthRoutes();
            
            console.log(JSON.stringify({
                timestamp: new Date().toISOString(),
                level: 'INFO',
                message: 'Lambda RIC initialization completed successfully',
                handler: this.functionHandler
            }));
            
            return true;
        } catch (error) {
            console.error(JSON.stringify({
                timestamp: new Date().toISOString(),
                level: 'ERROR',
                message: 'Lambda RIC initialization failed',
                error: {
                    type: error.constructor.name,
                    message: error.message,
                    stack: error.stack
                }
            }));
            process.exit(1);
        }
    }

    setupHealthRoutes() {
        this.app.get('/health', (req, res) => {
            res.json({ 
                status: 'ok',
                timestamp: new Date().toISOString(),
                service: 'lambda-ric-server'
            });
        });
    }

    async startPolling() {
        console.log(JSON.stringify({
            timestamp: new Date().toISOString(),
            level: 'INFO',
            message: 'Starting Lambda RIC polling loop',
            endpoint: this.lambdaRuntimeEndpoint
        }));

        while (!this.isShuttingDown) {
            try {
                const invocation = await this.pollForInvocation();
                if (invocation) {
                    await this.processInvocation(invocation);
                } else {
                    // No invocation available, wait a bit
                    await new Promise(resolve => setTimeout(resolve, 100));
                }
            } catch (error) {
                console.error(JSON.stringify({
                    timestamp: new Date().toISOString(),
                    level: 'ERROR',
                    message: 'Error in polling loop',
                    error: {
                        type: error.constructor.name,
                        message: error.message
                    }
                }));
                // Wait longer on error
                await new Promise(resolve => setTimeout(resolve, 1000));
            }
        }
    }

    async pollForInvocation() {
        try {
            const url = `${this.lambdaRuntimeEndpoint}/runtime/invocation/next`;
            const headers = {
                'lambda-runtime-aws-request-id': this.containerId
            };

            const response = await axios.get(url, { 
                headers,
                timeout: 30000,
                validateStatus: (status) => status === 200 || status === 204
            });

            if (response.status === 200) {
                return response.data;
            } else if (response.status === 204) {
                return null; // No invocation available
            }
        } catch (error) {
            if (error.response && error.response.status === 204) {
                return null; // No invocation available
            }
            throw error;
        }
    }

    async processInvocation(invocation) {
        const { request_id, payload, deadline_ms } = invocation;
        
        console.log(JSON.stringify({
            timestamp: new Date().toISOString(),
            level: 'INFO',
            message: 'Processing Lambda invocation',
            requestId: request_id
        }));

        try {
            // Create Lambda context object
            const context = {
                requestId: request_id,
                functionName: process.env.AWS_LAMBDA_FUNCTION_NAME || 'unknown',
                functionVersion: process.env.AWS_LAMBDA_FUNCTION_VERSION || '$LATEST',
                invokedFunctionArn: invocation.invoked_function_arn,
                memoryLimitInMB: process.env.AWS_LAMBDA_FUNCTION_MEMORY_SIZE || '128',
                remainingTimeInMillis: deadline_ms > 0 ? deadline_ms - Date.now() : 300000,
                deadlineMs: deadline_ms,
                traceId: invocation.trace_id,
                clientContext: invocation.client_context,
                cognitoIdentity: invocation.cognito_identity
            };

            // Execute the user function
            const result = await this.handler(payload, context);

            // Submit successful response
            await this.submitResponse(request_id, result);

            console.log(JSON.stringify({
                timestamp: new Date().toISOString(),
                level: 'INFO',
                message: 'Lambda invocation completed successfully',
                requestId: request_id
            }));

        } catch (error) {
            console.error(JSON.stringify({
                timestamp: new Date().toISOString(),
                level: 'ERROR',
                message: 'Lambda invocation failed',
                requestId: request_id,
                error: {
                    type: error.constructor.name,
                    message: error.message,
                    stack: error.stack
                }
            }));

            // Submit error response
            await this.submitError(request_id, error);
        }
    }

    async submitResponse(requestId, result) {
        try {
            const url = `${this.lambdaRuntimeEndpoint}/runtime/invocation/${requestId}/response`;
            await axios.post(url, result, {
                headers: { 'Content-Type': 'application/json' },
                timeout: 5000
            });
        } catch (error) {
            console.error(JSON.stringify({
                timestamp: new Date().toISOString(),
                level: 'ERROR',
                message: 'Failed to submit response',
                requestId: request_id,
                error: error.message
            }));
        }
    }

    async submitError(requestId, error) {
        try {
            const url = `${this.lambdaRuntimeEndpoint}/runtime/invocation/${requestId}/error`;
            const errorPayload = {
                errorType: error.constructor.name,
                errorMessage: error.message,
                stackTrace: error.stack ? error.stack.split('\n') : []
            };
            
            await axios.post(url, errorPayload, {
                headers: { 'Content-Type': 'application/json' },
                timeout: 5000
            });
        } catch (submitError) {
            console.error(JSON.stringify({
                timestamp: new Date().toISOString(),
                level: 'ERROR',
                message: 'Failed to submit error',
                requestId: request_id,
                error: submitError.message
            }));
        }
    }

    handleShutdown() {
        console.log(JSON.stringify({
            timestamp: new Date().toISOString(),
            level: 'INFO',
            message: 'Lambda RIC shutdown signal received'
        }));

        this.isShuttingDown = true;
    }

    async start(port = 8080) {
        await this.init();
        
        // Start health check server
        this.app.listen(port, () => {
            console.log(JSON.stringify({
                timestamp: new Date().toISOString(),
                level: 'INFO',
                message: `Lambda RIC health server listening on port ${port}`
            }));
        });

        // Start polling loop
        await this.startPolling();
    }
}

// Start the runtime if this is the main module
if (require.main === module) {
    const runtime = new LambdaRuntimeInterface();
    runtime.start();
}

module.exports = LambdaRuntimeInterface;