/**
 * Test Client - HTTP client for Lambda@Home API testing
 */

const axios = require('axios');

class TestClient {
    constructor(baseUrl = 'http://127.0.0.1:9000') {
        this.baseUrl = baseUrl;
        this.client = axios.create({
            baseURL: baseUrl,
            timeout: 30000,
            headers: {
                'Content-Type': 'application/json'
            },
            // Prevent connection pooling to avoid open handles
            httpAgent: new (require('http').Agent)({ keepAlive: false }),
            httpsAgent: new (require('https').Agent)({ keepAlive: false })
        });
    }

    async healthCheck() {
        try {
            const response = await this.client.get('/api/healthz');
            return { healthy: response.status === 200, status: response.status };
        } catch (error) {
            return { healthy: false, error: error.message };
        }
    }

    async createFunction(functionName, runtime = 'nodejs22.x', handler = 'index.handler', zipData, options = {}) {
        const payload = {
            function_name: functionName,
            runtime: runtime,
            handler: handler,
            code: {
                zip_file: zipData
            },
            description: `Test function for ${functionName}`,
            timeout: options.timeout || 30,
            memory_size: options.memory_size || 512
        };

        const response = await this.client.post('/api/2015-03-31/functions', payload);
        return response.data;
    }

    async invokeFunction(functionName, payload, headers = {}) {
        const invokeHeaders = {
            'X-Amz-Invocation-Type': 'RequestResponse',
            ...headers
        };

        const response = await this.client.post(
            `/api/2015-03-31/functions/${functionName}/invocations`,
            payload,
            { headers: invokeHeaders }
        );
        return response.data;
    }

    async getFunction(functionName) {
        const response = await this.client.get(`/api/2015-03-31/functions/${functionName}`);
        return response.data;
    }

    async listFunctions() {
        const response = await this.client.get('/api/2015-03-31/functions');
        return response.data;
    }

    async deleteFunction(functionName) {
        try {
            await this.client.delete(`/api/2015-03-31/functions/${functionName}`);
            return { success: true };
        } catch (error) {
            return { success: false, error: error.message };
        }
    }

    async getMetrics() {
        const response = await this.client.get('/api/metrics');
        return response.data;
    }

    async getWarmPool(functionName) {
        const response = await this.client.get(`/api/admin/warm-pool/${functionName}`);
        return response.data;
    }

    async createApiRoute(path, method, functionName) {
        const payload = {
            path: path,
            method: method,
            function_name: functionName
        };

        const response = await this.client.post('/api/admin/api-gateway/routes', payload);
        return response.data;
    }

    async invokeViaProxy(path, payload, headers = {}) {
        const response = await this.client.post(path, payload, { headers });
        return {
            status: response.status,
            data: response.data,
            headers: response.headers
        };
    }

    // Clean up HTTP connections
    close() {
        if (this.client && this.client.defaults.httpAgent) {
            this.client.defaults.httpAgent.destroy();
        }
        if (this.client && this.client.defaults.httpsAgent) {
            this.client.defaults.httpsAgent.destroy();
        }
    }
}

module.exports = TestClient;
