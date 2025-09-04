# API Caller Node.js Example

This is a Lambda function example that demonstrates how to make HTTP API calls from within a Lambda function.

## Features

- Makes HTTP/HTTPS requests to external APIs
- Supports GET, POST, PUT, PATCH, DELETE methods
- Configurable headers and request body
- Timeout handling
- Error handling with detailed responses
- JSON response parsing with fallback to plain text

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
  "timeout": 5000
}
```

### Parameters

- `url` (required): The URL to make the request to
- `method` (optional): HTTP method (GET, POST, PUT, PATCH, DELETE). Defaults to GET
- `headers` (optional): Object containing HTTP headers
- `timeout` (optional): Request timeout in milliseconds. Defaults to 5000ms

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
- HTTP errors (4xx, 5xx)
- Invalid JSON responses
- Connection failures

All errors are returned with detailed error messages and the original request for debugging.
