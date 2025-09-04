import json
import logging
import urllib.request
import urllib.parse
import urllib.error
import ssl
from typing import Dict, Any, Optional

logger = logging.getLogger()
logger.setLevel(logging.INFO)

def handler(event, context):
    """
    Lambda function that makes HTTP API calls
    
    Expected event format:
    {
        "url": "https://httpbin.org/get",  # optional, defaults to httpbin.org/get
        "method": "GET",  # optional, defaults to GET
        "headers": {"User-Agent": "lambda-at-home"},  # optional
        "data": "request body",  # optional, for POST/PUT requests
        "timeout": 5  # optional, defaults to 5 seconds
    }
    """
    logger.info("API caller event: %s", json.dumps(event, indent=2))
    
    try:
        # Parse the event
        url = event.get('url')
        method = event.get('method', 'GET').upper()
        headers = event.get('headers', {})
        data = event.get('data')
        timeout = event.get('timeout', 5)
        
        # If no URL provided, use a default test endpoint
        target_url = url or 'https://httpbin.org/get'
        
        logger.info("Making %s request to: %s", method, target_url)
        
        # Make the API call
        response = make_http_request(target_url, method, headers, data, timeout)
        
        return {
            'statusCode': 200,
            'body': json.dumps({
                'success': True,
                'request': {
                    'url': target_url,
                    'method': method,
                    'headers': headers
                },
                'response': response,
                'note': 'Custom URL provided' if url else 'Using default test URL (httpbin.org/get)'
            })
        }
        
    except Exception as error:
        logger.error('API call failed: %s', str(error))
        
        return {
            'statusCode': 500,
            'body': json.dumps({
                'success': False,
                'error': str(error),
                'request': event
            })
        }

def make_http_request(url: str, method: str, headers: Dict[str, str], 
                     data: Optional[str], timeout: int) -> Dict[str, Any]:
    """
    Make an HTTP request and return the response
    """
    # Prepare headers
    request_headers = {
        'User-Agent': 'lambda-at-home-api-caller',
        **headers
    }
    
    # Prepare request data
    request_data = None
    if data and method in ['POST', 'PUT', 'PATCH']:
        if isinstance(data, str):
            request_data = data.encode('utf-8')
        else:
            request_data = json.dumps(data).encode('utf-8')
            request_headers['Content-Type'] = 'application/json'
    
    # Create request
    req = urllib.request.Request(url, data=request_data, headers=request_headers, method=method)
    
    # Create SSL context that doesn't verify certificates (for testing)
    # In production, you might want to verify certificates
    ssl_context = ssl.create_default_context()
    ssl_context.check_hostname = False
    ssl_context.verify_mode = ssl.CERT_NONE
    
    try:
        with urllib.request.urlopen(req, timeout=timeout, context=ssl_context) as response:
            response_data = response.read().decode('utf-8')
            
            # Try to parse as JSON, fallback to string
            try:
                parsed_data = json.loads(response_data)
            except json.JSONDecodeError:
                parsed_data = response_data
            
            return {
                'statusCode': response.status,
                'headers': dict(response.headers),
                'data': parsed_data
            }
            
    except urllib.error.HTTPError as e:
        # Handle HTTP errors (4xx, 5xx)
        error_data = e.read().decode('utf-8') if e.fp else ''
        try:
            parsed_error = json.loads(error_data)
        except json.JSONDecodeError:
            parsed_error = error_data
            
        return {
            'statusCode': e.code,
            'headers': dict(e.headers) if hasattr(e, 'headers') else {},
            'data': parsed_error,
            'error': True
        }
        
    except urllib.error.URLError as e:
        raise Exception(f"URL error: {e.reason}")
    except Exception as e:
        raise Exception(f"Request failed: {str(e)}")
