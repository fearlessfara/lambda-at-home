import os
import sys
import json
import signal
import logging
import importlib.util
from datetime import datetime
from typing import Any, Dict, Optional
from base64 import b64encode, b64decode
from flask import Flask, request, jsonify

class RuntimeInterface:
    def __init__(self):
        self.app = Flask(__name__)
        self.handler = None
        self.is_shutting_down = False
        
        # Configure logging
        logging.basicConfig(format='%(message)s', level=logging.INFO)
        
        # Setup signal handlers
        signal.signal(signal.SIGTERM, self.handle_shutdown)
        signal.signal(signal.SIGINT, self.handle_shutdown)
        
        # Setup routes
        self.setup_routes()

    def log(self, level: str, message: str, **kwargs):
        """Emit structured log entry"""
        log_entry = {
            "timestamp": datetime.utcnow().isoformat(),
            "level": level.upper(),
            "message": message
        }
        log_entry.update(kwargs)
        logging.info(json.dumps(log_entry))

    def init(self) -> bool:
        """Initialize the runtime and load the handler"""
        try:
            handler_string = os.environ.get('HANDLER', 'index.handler')
            self.log("info", "RIC initialization started", handler=handler_string)
            
            # Parse handler string
            module_name, function_name = handler_string.split('.')
            
            # Load user code
            module_path = os.path.join(os.getcwd(), f"{module_name}.py")
            spec = importlib.util.spec_from_file_location(module_name, module_path)
            if not spec or not spec.loader:
                raise ImportError(f"Could not load module: {module_name}")
                
            module = importlib.util.module_from_spec(spec)
            sys.modules[module_name] = module
            spec.loader.exec_module(module)
            
            # Get handler function
            self.handler = getattr(module, function_name)
            if not callable(self.handler):
                raise ValueError(f"Handler {handler_string} is not callable")
            
            self.log("info", "RIC initialization completed successfully", handler=handler_string)
            return True
            
        except Exception as e:
            self.log("error", "Initialization failed",
                    error={
                        "type": e.__class__.__name__,
                        "message": str(e),
                        "stack": self.get_stack_trace(e)
                    })
            sys.exit(1)

    def get_stack_trace(self, error: Exception) -> list:
        """Extract formatted stack trace from exception"""
        import traceback
        return traceback.format_exception(type(error), error, error.__traceback__)

    def setup_routes(self):
        @self.app.route('/health', methods=['GET'])
        def health():
            return jsonify({"status": "ok"})

        @self.app.route('/invoke', methods=['POST'])
        def invoke():
            if self.is_shutting_down:
                return jsonify({
                    "status_code": 503,
                    "body": {
                        "errorMessage": "Service is shutting down",
                        "errorType": "ServiceUnavailable"
                    },
                    "logs": []
                }), 503

            start_time = datetime.now().timestamp() * 1000
            deadline_ms = int(request.headers.get('x-deadline-ms', 0))
            request_id = request.headers.get('x-request-id', 'unknown')

            self.log("info", "Function execution started", 
                    requestId=request_id, deadlineMs=deadline_ms)

            try:
                # Parse request body
                event = request.get_json()
                
                # Pass the raw payload to the handler
                # (Handler can decode base64 if needed)

                # Create context object
                context = {
                    "deadlineMs": deadline_ms - int((datetime.now().timestamp() * 1000) - start_time) if deadline_ms > 0 else 0,
                    "requestId": request_id
                }

                # Invoke handler
                result = self.handler(event, context)

                execution_time = int((datetime.now().timestamp() * 1000) - start_time)
                self.log("info", "Function execution completed successfully",
                        requestId=request_id, executionTimeMs=execution_time)

                # Handle base64 response if needed
                if isinstance(result, bytes):
                    return jsonify({
                        "status_code": 200,
                        "body": {
                            "base64": True,
                            "data": b64encode(result).decode()
                        },
                        "logs": []
                    })
                elif isinstance(result, dict) and result.get('base64'):
                    return jsonify({
                        "status_code": 200,
                        "body": result,
                        "logs": []
                    })
                else:
                    return jsonify({
                        "status_code": 200,
                        "body": result,
                        "logs": []
                    })

            except Exception as e:
                execution_time = int((datetime.now().timestamp() * 1000) - start_time)
                self.log("error", "Function execution failed",
                        requestId=request_id, executionTimeMs=execution_time,
                        error={
                            "type": e.__class__.__name__,
                            "message": str(e),
                            "stack": self.get_stack_trace(e)
                        })
                
                return jsonify({
                    "status_code": 500,
                    "body": {
                        "errorMessage": str(e),
                        "errorType": e.__class__.__name__,
                        "stackTrace": self.get_stack_trace(e)
                    },
                    "logs": []
                }), 500

    def handle_shutdown(self, signum, frame):
        """Handle shutdown signals"""
        self.log("info", "Container termination signal received")
        self.is_shutting_down = True

    def start(self, port: int = 8080):
        """Start the runtime server"""
        if self.init():
            self.log("info", f"Runtime Interface listening on port {port}")
            self.app.run(host='0.0.0.0', port=port)

if __name__ == '__main__':
    runtime = RuntimeInterface()
    runtime.start()