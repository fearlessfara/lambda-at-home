const axios = require('axios');
const fs = require('fs');
const path = require('path');

async function test() {
    const client = axios.create({
        baseURL: 'http://127.0.0.1:8000',
        timeout: 30000
    });

    // Load test function
    const zipPath = path.join(__dirname, 'test-function.zip');
    const zipData = fs.readFileSync(zipPath).toString('base64');

    // Create function
    console.log('Creating nodejs24.x function...');
    const createResp = await client.post('/2015-03-31/functions', {
        function_name: 'test-nodejs24-' + Date.now(),
        runtime: 'nodejs24.x',
        handler: 'index.handler',
        code: { zip_file: zipData }
    });
    console.log('Function created:', createResp.data.function_name);

    // Wait a bit for function to be ready
    await new Promise(r => setTimeout(r, 3000));

    // Invoke function
    console.log('Invoking function...');
    const invokeResp = await client.post(`/2015-03-31/functions/${createResp.data.function_name}/invocations`, {
        testId: 'test',
        message: 'Hello Node 24!'
    });
    console.log('✅ Result:', JSON.stringify(invokeResp.data, null, 2));
}

test().catch(err => {
    console.error('❌ Error:', err.response?.data || err.message);
    process.exit(1);
});
