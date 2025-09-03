const RuntimeInterface = require('../src/index');
const request = require('supertest');
const path = require('path');

describe('Runtime Interface Client', () => {
    let runtime;
    
    beforeEach(() => {
        process.env.HANDLER = 'test-handler.handler';
        runtime = new RuntimeInterface();
    });

    test('initializes with valid handler', async () => {
        // Mock require for test handler
        jest.mock(path.join(process.cwd(), 'test-handler'), () => ({
            handler: async (event, context) => ({ success: true })
        }), { virtual: true });

        const result = await runtime.init();
        expect(result).toBe(true);
    });

    test('handles regular JSON payload', async () => {
        // Mock handler
        jest.mock(path.join(process.cwd(), 'test-handler'), () => ({
            handler: async (event, context) => event
        }), { virtual: true });

        await runtime.init();
        
        const payload = { test: 'data' };
        const response = await request(runtime.app)
            .post('/invoke')
            .send(payload)
            .set('x-request-id', 'test-123')
            .set('x-deadline-ms', '3000');

        expect(response.status).toBe(200);
        expect(response.body).toEqual(payload);
    });

    test('handles base64 encoded payload', async () => {
        // Mock handler
        jest.mock(path.join(process.cwd(), 'test-handler'), () => ({
            handler: async (event, context) => event
        }), { virtual: true });

        await runtime.init();
        
        const originalData = { test: 'data' };
        const base64Data = Buffer.from(JSON.stringify(originalData)).toString('base64');
        
        const response = await request(runtime.app)
            .post('/invoke')
            .send({ base64: true, data: base64Data });

        expect(response.status).toBe(200);
        expect(response.body).toEqual(originalData);
    });

    test('handles handler errors', async () => {
        // Mock handler that throws
        jest.mock(path.join(process.cwd(), 'test-handler'), () => ({
            handler: async () => {
                throw new Error('Test error');
            }
        }), { virtual: true });

        await runtime.init();
        
        const response = await request(runtime.app)
            .post('/invoke')
            .send({ test: 'data' });

        expect(response.status).toBe(500);
        expect(response.body).toHaveProperty('errorMessage', 'Test error');
        expect(response.body).toHaveProperty('errorType', 'Error');
        expect(response.body).toHaveProperty('stackTrace');
    });

    test('respects shutdown signal', async () => {
        await runtime.init();
        runtime.handleShutdown();
        
        const response = await request(runtime.app)
            .post('/invoke')
            .send({ test: 'data' });

        expect(response.status).toBe(503);
        expect(response.body.errorType).toBe('ServiceUnavailable');
    });
});
