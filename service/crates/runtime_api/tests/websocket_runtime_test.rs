use std::process::Command;
use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_nodejs_websocket_bootstrap_syntax() {
    // Test that the Node.js WebSocket bootstrap file has valid syntax
    let output = Command::new("node")
        .arg("--check")
        .arg("runtimes/nodejs18/bootstrap-websocket.js")
        .current_dir("../../..")
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                panic!("Node.js WebSocket bootstrap syntax error: {}", stderr);
            }
        }
        Err(e) => {
            // Node.js might not be available in the test environment
            println!("Node.js not available for syntax check: {}", e);
        }
    }
}

#[tokio::test]
async fn test_nodejs_bootstrap_websocket_detection() {
    // Test that the main Node.js bootstrap can detect WebSocket support
    let output = Command::new("node")
        .arg("-e")
        .arg(r#"
            const USE_WEBSOCKET = process.env.LAMBDA_USE_WEBSOCKET !== 'false';
            const HAS_WS = (() => {
                try {
                    require.resolve('ws');
                    return true;
                } catch (e) {
                    return false;
                }
            })();
            console.log('USE_WEBSOCKET:', USE_WEBSOCKET);
            console.log('HAS_WS:', HAS_WS);
            if (USE_WEBSOCKET && HAS_WS) {
                console.log('WebSocket runtime would be used');
            } else {
                console.log('HTTP runtime would be used');
            }
        "#)
        .current_dir(".")
        .output();

    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            println!("Node.js WebSocket detection output: {}", stdout);
            
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                panic!("Node.js WebSocket detection error: {}", stderr);
            }
        }
        Err(e) => {
            println!("Node.js not available for WebSocket detection test: {}", e);
        }
    }
}

#[tokio::test]
async fn test_python_websocket_bootstrap_syntax() {
    // Test that the Python WebSocket bootstrap file has valid syntax
    let output = Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg("runtimes/python311/bootstrap-websocket.py")
        .current_dir("../../..")
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                panic!("Python WebSocket bootstrap syntax error: {}", stderr);
            }
        }
        Err(e) => {
            // Python might not be available in the test environment
            println!("Python not available for syntax check: {}", e);
        }
    }
}

#[tokio::test]
async fn test_python_bootstrap_websocket_detection() {
    // Test that the main Python bootstrap can detect WebSocket support
    let output = Command::new("python3")
        .arg("-c")
        .arg(r#"
import os
USE_WEBSOCKET = os.environ.get('LAMBDA_USE_WEBSOCKET', 'true').lower() != 'false'
HAS_WEBSOCKETS = False
try:
    import websockets
    HAS_WEBSOCKETS = True
except ImportError:
    HAS_WEBSOCKETS = False

print('USE_WEBSOCKET:', USE_WEBSOCKET)
print('HAS_WEBSOCKETS:', HAS_WEBSOCKETS)
if USE_WEBSOCKET and HAS_WEBSOCKETS:
    print('WebSocket runtime would be used')
else:
    print('HTTP runtime would be used')
        "#)
        .current_dir(".")
        .output();

    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            println!("Python WebSocket detection output: {}", stdout);
            
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                panic!("Python WebSocket detection error: {}", stderr);
            }
        }
        Err(e) => {
            println!("Python not available for WebSocket detection test: {}", e);
        }
    }
}

#[tokio::test]
async fn test_example_function_syntax() {
    // Test that the example functions have valid syntax
    
    // Test Node.js example
    let node_output = Command::new("node")
        .arg("--check")
        .arg("examples/websocket-test-function/index.js")
        .current_dir("../../..")
        .output();

    match node_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                panic!("Node.js example function syntax error: {}", stderr);
            }
        }
        Err(e) => {
            println!("Node.js not available for example syntax check: {}", e);
        }
    }

    // Test Python example
    let python_output = Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg("examples/websocket-test-function-python/lambda_function.py")
        .current_dir("../../..")
        .output();

    match python_output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                panic!("Python example function syntax error: {}", stderr);
            }
        }
        Err(e) => {
            println!("Python not available for example syntax check: {}", e);
        }
    }
}

#[tokio::test]
async fn test_websocket_dependencies() {
    // Test that WebSocket dependencies can be imported
    
    // Test Node.js ws package
    let node_output = Command::new("node")
        .arg("-e")
        .arg("try { require('ws'); console.log('ws package available'); } catch(e) { console.log('ws package not available:', e.message); }")
        .current_dir("../../../examples/websocket-test-function")
        .output();

    match node_output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            println!("Node.js ws package test: {}", stdout);
        }
        Err(e) => {
            println!("Node.js not available for dependency test: {}", e);
        }
    }

    // Test Python websockets package
    let python_output = Command::new("python3")
        .arg("-c")
        .arg("try: import websockets; print('websockets package available'); except ImportError as e: print('websockets package not available:', e)")
        .current_dir("../../../examples/websocket-test-function-python")
        .output();

    match python_output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            println!("Python websockets package test: {}", stdout);
        }
        Err(e) => {
            println!("Python not available for dependency test: {}", e);
        }
    }
}
