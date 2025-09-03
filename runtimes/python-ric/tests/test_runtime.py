import os
import json
import pytest
import base64
from unittest.mock import patch
from src.runtime import RuntimeInterface

@pytest.fixture
def runtime():
    os.environ['HANDLER'] = 'test_handler.handler'
    return RuntimeInterface()

@pytest.fixture
def test_client(runtime):
    with runtime.app.test_client() as client:
        yield client

def test_init_with_valid_handler(tmp_path, runtime):
    # Create a temporary handler file
    handler_path = tmp_path / "test_handler.py"
    handler_path.write_text("""
def handler(event, context):
    return {"success": True}
""")
    
    with patch('os.getcwd', return_value=str(tmp_path)):
        assert runtime.init() is True

def test_handle_json_payload(tmp_path, runtime, test_client):
    # Create a temporary handler file
    handler_path = tmp_path / "test_handler.py"
    handler_path.write_text("""
def handler(event, context):
    return event
""")
    
    with patch('os.getcwd', return_value=str(tmp_path)):
        runtime.init()
        
        payload = {"test": "data"}
        response = test_client.post('/invoke', 
                                  json=payload,
                                  headers={
                                      'x-request-id': 'test-123',
                                      'x-deadline-ms': '3000'
                                  })
        
        assert response.status_code == 200
        assert response.json == payload

def test_handle_base64_payload(tmp_path, runtime, test_client):
    # Create a temporary handler file
    handler_path = tmp_path / "test_handler.py"
    handler_path.write_text("""
def handler(event, context):
    return event
""")
    
    with patch('os.getcwd', return_value=str(tmp_path)):
        runtime.init()
        
        original_data = {"test": "data"}
        base64_data = base64.b64encode(json.dumps(original_data).encode()).decode()
        
        response = test_client.post('/invoke',
                                  json={"base64": True, "data": base64_data})
        
        assert response.status_code == 200
        assert response.json == original_data

def test_handle_errors(tmp_path, runtime, test_client):
    # Create a temporary handler file
    handler_path = tmp_path / "test_handler.py"
    handler_path.write_text("""
def handler(event, context):
    raise ValueError("Test error")
""")
    
    with patch('os.getcwd', return_value=str(tmp_path)):
        runtime.init()
        
        response = test_client.post('/invoke', json={"test": "data"})
        
        assert response.status_code == 500
        assert response.json["errorMessage"] == "Test error"
        assert response.json["errorType"] == "ValueError"
        assert isinstance(response.json["stackTrace"], list)

def test_shutdown_handling(runtime, test_client):
    runtime.init()
    runtime.handle_shutdown(None, None)
    
    response = test_client.post('/invoke', json={"test": "data"})
    
    assert response.status_code == 503
    assert response.json["errorType"] == "ServiceUnavailable"
