# API Caller Python Example

This is a Lambda function example that demonstrates how to make HTTP API calls from within a Python Lambda function using only the standard library.

## Features

- Makes HTTP/HTTPS requests to external APIs using `urllib`
- Supports GET, POST, PUT, PATCH, DELETE methods
- Configurable headers and request body
- Timeout handling
- Comprehensive error handling
- JSON response parsing with fallback to plain text
- SSL context configuration for testing

## Usage

### Event Format

```json
{
  "url": "https://httpbin.org/get",
  "method": "GET",
  "headers": {
    "User-Agent": "lambda-at-home",
    "Authorization": "Bearer your-token"
  },
  "data": "request body",
  "timeout": 5
}
```

### Parameters

- `url` (required): The URL to make the request to
- `method` (optional): HTTP method (GET, POST, PUT, PATCH, DELETE). Defaults to GET
- `headers` (optional): Object containing HTTP headers
- `data` (optional): Request body for POST/PUT/PATCH requests
- `timeout` (optional): Request timeout in seconds. Defaults to 5 seconds

### Response Format

```json
{
  "statusCode": 200,
  "body": {
    "success": true,
    "request": {
      "url": "https://httpbin.org/get",
      "method": "GET",
      "headers": {...}
    },
    "response": {
      "statusCode": 200,
      "headers": {...},
      "data": {...}
    }
  }
}
```

## Example Invocations

### Simple GET Request
```json
{
  "url": "https://httpbin.org/get"
}
```

### POST Request with JSON Data
```json
{
  "url": "https://httpbin.org/post",
  "method": "POST",
  "headers": {
    "Content-Type": "application/json"
  },
  "data": {
    "message": "Hello from Lambda!"
  }
}
```

### Request with Custom Headers
```json
{
  "url": "https://api.github.com/user",
  "method": "GET",
  "headers": {
    "Authorization": "token your-github-token",
    "Accept": "application/vnd.github.v3+json"
  }
}
```

## Testing

You can test this function with various APIs:

- **httpbin.org**: Great for testing HTTP requests
- **jsonplaceholder.typicode.com**: REST API for testing
- **api.github.com**: GitHub API (requires authentication)
- **httpstat.us**: Returns specific HTTP status codes

## Error Handling

The function handles various error scenarios:

- Missing URL parameter (400 error)
- Network timeouts
- HTTP errors (4xx, 5xx) - returns the error response
- Invalid JSON responses
- Connection failures
- SSL/TLS errors

All errors are returned with detailed error messages and the original request for debugging.

## Security Notes

- The function uses a custom SSL context that doesn't verify certificates (for testing)
- In production, you should enable certificate verification
- Be careful with sensitive data in headers and request bodies
- Consider using environment variables for API keys and tokens
