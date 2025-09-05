const https = require('https');
const http = require('http');

exports.handler = async (event) => {
  console.log("API caller event:", JSON.stringify(event, null, 2));
  
  try {
    // Parse the event to get API configuration
    const { url, method = 'GET', headers = {}, timeout = 5000 } = event;
    
    // If no URL provided, use a default test endpoint
    const targetUrl = url || 'https://httpbin.org/get';
    
    console.log(`Making ${method} request to: ${targetUrl}`);

    // Make the API call
    const result = await makeHttpRequest(targetUrl, method, headers, timeout);
    
    return {
      statusCode: 200,
      body: JSON.stringify({
        success: true,
        request: { url: targetUrl, method, headers },
        response: result,
        note: url ? 'Custom URL provided' : 'Using default test URL (httpbin.org/get)'
      })
    };
    
  } catch (error) {
    console.error('API call failed:', error);
    
    return {
      statusCode: 500,
      body: JSON.stringify({
        success: false,
        error: error.message,
        request: event
      })
    };
  }
};

function makeHttpRequest(url, method, headers, timeout) {
  return new Promise((resolve, reject) => {
    const urlObj = new URL(url);
    const isHttps = urlObj.protocol === 'https:';
    const client = isHttps ? https : http;
    
    // Sanitize incoming headers: never forward Host/host to a different origin
    const sanitizedHeaders = { 'User-Agent': 'lambda-at-home-api-caller', ...headers };
    delete sanitizedHeaders.host; delete sanitizedHeaders.Host; delete sanitizedHeaders[":authority"]; // in case

    const options = {
      hostname: urlObj.hostname,
      port: urlObj.port || (isHttps ? 443 : 80),
      path: urlObj.pathname + urlObj.search,
      method: method.toUpperCase(),
      headers: sanitizedHeaders,
      servername: urlObj.hostname, // ensure SNI/verification uses target host
      timeout: timeout
    };

    const req = client.request(options, (res) => {
      let data = '';
      
      res.on('data', (chunk) => {
        data += chunk;
      });
      
      res.on('end', () => {
        try {
          // Try to parse as JSON, fallback to string
          let parsedData;
          try {
            parsedData = JSON.parse(data);
          } catch {
            parsedData = data;
          }
          
          resolve({
            statusCode: res.statusCode,
            headers: res.headers,
            data: parsedData
          });
        } catch (error) {
          reject(new Error(`Failed to process response: ${error.message}`));
        }
      });
    });

    req.on('error', (error) => {
      reject(new Error(`Request failed: ${error.message}`));
    });

    req.on('timeout', () => {
      req.destroy();
      reject(new Error(`Request timeout after ${timeout}ms`));
    });

    req.end();
  });
}
