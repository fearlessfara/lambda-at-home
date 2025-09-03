use lambda_at_home::{
    core::config::Config,
    docker::container_lifecycle::ContainerLifecycleManager,
    docker::docker::DockerManager,
    api::lambda_executor::LambdaExecutor,
    api::lambda_runtime_api::LambdaRuntimeService,
    core::models::{Function, FunctionStatus, RICInvokeRequest},
    core::storage::FunctionStorage,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_real_world_single_function() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Real-World Lambda Single Function Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::new());
    let lifecycle_manager = Arc::new(ContainerLifecycleManager::new(
        docker_manager.clone(),
        config.clone(),
        storage.clone(),
    ));

    // Create Lambda executor
    let executor = LambdaExecutor::new(
        docker_manager,
        lifecycle_manager,
        lambda_runtime_service.clone(),
    );

    // Create a test function using an existing function ID that has a Docker image
    let function_id = Uuid::parse_str("ad80c2e0-aedc-4809-a27b-317361ec87e6").unwrap();
    let function_name = format!("real-world-test-{}", function_id);
    
    info!("📝 Creating real-world test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Real-world test function for Lambda execution".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function_id)),
        memory_size: Some(128),
        cpu_limit: Some(0.5),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Store function metadata
    storage.save_function(&function_metadata).await.expect("Failed to store function");

    // Test payload
    let test_payload = json!({
        "message": "Hello from real-world Lambda test!",
        "number": 42,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "single_function"
    });

    let invoke_request = RICInvokeRequest {
        payload: test_payload.clone(),
        timeout: Some(30),
    };

    info!("🧪 Testing real-world Lambda function execution");

    // Execute the function
    let start_time = Instant::now();
    
    match executor.invoke_function(&function_id, &invoke_request).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            info!("✅ Real-world Lambda execution completed successfully in {:?}", duration);
            info!("📊 Response: {:?}", response);
            
            // Verify response structure
            assert_eq!(response.status_code, 200);
            assert!(response.payload.is_some());
            assert!(response.duration_ms.is_some());
            assert!(response.duration_ms.unwrap() > 0);
            
            let body = response.payload.unwrap();
            if let Some(body_obj) = body.as_object() {
                assert!(body_obj.contains_key("message"));
                assert!(body_obj.contains_key("received_payload"));
                assert!(body_obj.contains_key("handler"));
                assert!(body_obj.contains_key("timestamp"));
                assert!(body_obj.contains_key("computation_result"));
                
                // Verify computation result (42 * 42 = 1764)
                let computation_result = body_obj.get("computation_result").unwrap().as_f64().unwrap();
                assert_eq!(computation_result, 1764.0);
                
                // Verify the test type was passed through
                let received_payload = body_obj.get("received_payload").unwrap().as_object().unwrap();
                assert_eq!(received_payload.get("test_type").unwrap().as_str().unwrap(), "single_function");
            } else {
                panic!("Expected response body to be an object");
            }
            
            info!("✅ All real-world response validations passed!");
        }
        Err(e) => {
            warn!("❌ Real-world Lambda execution failed: {}", e);
            panic!("Real-world Lambda execution failed: {}", e);
        }
    }

    info!("🎉 Real-World Lambda Single Function Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_real_world_concurrent_execution() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Real-World Lambda Concurrent Execution Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::with_max_concurrency(3));
    let lifecycle_manager = Arc::new(ContainerLifecycleManager::new(
        docker_manager.clone(),
        config.clone(),
        storage.clone(),
    ));

    // Create Lambda executor
    let executor = LambdaExecutor::new(
        docker_manager,
        lifecycle_manager,
        lambda_runtime_service.clone(),
    );

    // Create a test function
    let function_id = Uuid::new_v4();
    let function_name = format!("concurrent-test-{}", function_id);
    
    info!("📝 Creating concurrent test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Concurrent test function for Lambda execution".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function_id)),
        memory_size: Some(128),
        cpu_limit: Some(0.5),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Store function metadata
    storage.save_function(&function_metadata).await.expect("Failed to store function");

    // Test concurrent invocations
    info!("🧪 Testing concurrent Lambda function execution");

    let num_concurrent = 3;
    let mut handles = Vec::new();

    for i in 0..num_concurrent {
        let executor_clone = executor.clone();
        let function_id_clone = function_id;
        
        let handle = tokio::spawn(async move {
            let test_payload = json!({
                "message": format!("Hello from concurrent invocation {}!", i),
                "number": i * 10,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "test_type": "concurrent_execution",
                "invocation_id": i
            });

            let invoke_request = RICInvokeRequest {
                payload: test_payload.clone(),
                timeout: Some(30),
            };

            info!("🚀 Starting concurrent invocation {}", i);
            let start_time = Instant::now();
            
            match executor_clone.invoke_function(&function_id_clone, &invoke_request).await {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    info!("✅ Concurrent invocation {} completed successfully in {:?}", i, duration);
                    
                    // Verify response structure
                    assert_eq!(response.status_code, 200);
                    assert!(response.payload.is_some());
                    
                    let body = response.payload.unwrap();
                    if let Some(body_obj) = body.as_object() {
                        assert!(body_obj.contains_key("message"));
                        assert!(body_obj.contains_key("received_payload"));
                        assert!(body_obj.contains_key("handler"));
                        assert!(body_obj.contains_key("timestamp"));
                        assert!(body_obj.contains_key("computation_result"));
                        
                        // Verify computation result (i * 10 * 42)
                        let computation_result = body_obj.get("computation_result").unwrap().as_f64().unwrap();
                        assert_eq!(computation_result, (i * 10 * 42) as f64);
                        
                        // Verify the invocation ID was passed through
                        let received_payload = body_obj.get("received_payload").unwrap().as_object().unwrap();
                        assert_eq!(received_payload.get("invocation_id").unwrap().as_f64().unwrap(), i as f64);
                    } else {
                        panic!("Expected response body to be an object");
                    }
                    
                    info!("✅ Concurrent invocation {} validations passed!", i);
                    Ok(())
                }
                Err(e) => {
                    warn!("❌ Concurrent invocation {} failed: {}", i, e);
                    Err(e)
                }
            }
        });
        
        handles.push(handle);
    }

    // Wait for all concurrent invocations to complete
    let mut results = Vec::new();
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(())) => {
                info!("✅ Concurrent invocation {} completed successfully", i);
                results.push(Ok(()));
            }
            Ok(Err(e)) => {
                warn!("❌ Concurrent invocation {} failed: {}", i, e);
                results.push(Err(e));
            }
            Err(e) => {
                warn!("❌ Concurrent invocation {} task failed: {}", i, e);
                results.push(Err(anyhow::anyhow!("Task failed: {}", e)));
            }
        }
    }

    // Verify all invocations succeeded
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Concurrent invocation {} failed: {:?}", i, result);
    }

    info!("🎉 All {} concurrent invocations completed successfully!", num_concurrent);
    info!("🎉 Real-World Lambda Concurrent Execution Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_real_world_multiple_functions() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Real-World Lambda Multiple Functions Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::new());
    let lifecycle_manager = Arc::new(ContainerLifecycleManager::new(
        docker_manager.clone(),
        config.clone(),
        storage.clone(),
    ));

    // Create Lambda executor
    let executor = LambdaExecutor::new(
        docker_manager,
        lifecycle_manager,
        lambda_runtime_service.clone(),
    );

    // Create multiple test functions
    let function1_id = Uuid::new_v4();
    let function2_id = Uuid::new_v4();
    
    info!("📝 Creating multiple test functions");

    // Create function 1 metadata
    let function1_metadata = Function {
        id: function1_id,
        name: format!("multi-function-1-{}", function1_id),
        description: Some("First function for multiple functions test".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function1_id)),
        memory_size: Some(128),
        cpu_limit: Some(0.5),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Create function 2 metadata
    let function2_metadata = Function {
        id: function2_id,
        name: format!("multi-function-2-{}", function2_id),
        description: Some("Second function for multiple functions test".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function2_id)),
        memory_size: Some(256),
        cpu_limit: Some(1.0),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Store function metadata
    storage.save_function(&function1_metadata).await.expect("Failed to store function 1");
    storage.save_function(&function2_metadata).await.expect("Failed to store function 2");

    // Test both functions
    info!("🧪 Testing multiple Lambda functions");

    // Test function 1
    let test_payload1 = json!({
        "message": "Hello from function 1!",
        "number": 10,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "multiple_functions",
        "function_id": 1
    });

    let invoke_request1 = RICInvokeRequest {
        payload: test_payload1.clone(),
        timeout: Some(30),
    };

    info!("🚀 Testing function 1");
    let start_time1 = Instant::now();
    
    match executor.invoke_function(&function1_id, &invoke_request1).await {
        Ok(response1) => {
            let duration1 = start_time1.elapsed();
            info!("✅ Function 1 execution completed successfully in {:?}", duration1);
            
            assert_eq!(response1.status_code, 200);
            assert!(response1.payload.is_some());
            
            let body1 = response1.payload.unwrap();
            if let Some(body_obj1) = body1.as_object() {
                let computation_result1 = body_obj1.get("computation_result").unwrap().as_f64().unwrap();
                assert_eq!(computation_result1, 420.0); // 10 * 42
                
                let received_payload1 = body_obj1.get("received_payload").unwrap().as_object().unwrap();
                assert_eq!(received_payload1.get("function_id").unwrap().as_f64().unwrap(), 1.0);
            }
            
            info!("✅ Function 1 validations passed!");
        }
        Err(e) => {
            warn!("❌ Function 1 execution failed: {}", e);
            panic!("Function 1 execution failed: {}", e);
        }
    }

    // Test function 2
    let test_payload2 = json!({
        "message": "Hello from function 2!",
        "number": 20,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "multiple_functions",
        "function_id": 2
    });

    let invoke_request2 = RICInvokeRequest {
        payload: test_payload2.clone(),
        timeout: Some(30),
    };

    info!("🚀 Testing function 2");
    let start_time2 = Instant::now();
    
    match executor.invoke_function(&function2_id, &invoke_request2).await {
        Ok(response2) => {
            let duration2 = start_time2.elapsed();
            info!("✅ Function 2 execution completed successfully in {:?}", duration2);
            
            assert_eq!(response2.status_code, 200);
            assert!(response2.payload.is_some());
            
            let body2 = response2.payload.unwrap();
            if let Some(body_obj2) = body2.as_object() {
                let computation_result2 = body_obj2.get("computation_result").unwrap().as_f64().unwrap();
                assert_eq!(computation_result2, 840.0); // 20 * 42
                
                let received_payload2 = body_obj2.get("received_payload").unwrap().as_object().unwrap();
                assert_eq!(received_payload2.get("function_id").unwrap().as_f64().unwrap(), 2.0);
            }
            
            info!("✅ Function 2 validations passed!");
        }
        Err(e) => {
            warn!("❌ Function 2 execution failed: {}", e);
            panic!("Function 2 execution failed: {}", e);
        }
    }

    info!("🎉 Real-World Lambda Multiple Functions Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_real_world_error_handling() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Real-World Lambda Error Handling Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::new());
    let lifecycle_manager = Arc::new(ContainerLifecycleManager::new(
        docker_manager.clone(),
        config.clone(),
        storage.clone(),
    ));

    // Create Lambda executor
    let executor = LambdaExecutor::new(
        docker_manager,
        lifecycle_manager,
        lambda_runtime_service.clone(),
    );

    // Create a test function
    let function_id = Uuid::new_v4();
    let function_name = format!("error-test-{}", function_id);
    
    info!("📝 Creating error test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Error test function for Lambda execution".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function_id)),
        memory_size: Some(128),
        cpu_limit: Some(0.5),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Store function metadata
    storage.save_function(&function_metadata).await.expect("Failed to store function");

    // Test error handling with invalid function ID
    info!("🧪 Testing error handling with invalid function ID");

    let invalid_function_id = Uuid::new_v4();
    let test_payload = json!({
        "message": "This should fail",
        "number": 42,
    });

    let invoke_request = RICInvokeRequest {
        payload: test_payload,
        timeout: Some(30),
    };

    // This should fail because the function doesn't exist
    match executor.invoke_function(&invalid_function_id, &invoke_request).await {
        Ok(_) => {
            panic!("Expected function invocation to fail for invalid function ID");
        }
        Err(e) => {
            info!("✅ Correctly failed for invalid function ID: {}", e);
            assert!(e.to_string().contains("function") || e.to_string().contains("not found"));
        }
    }

    // Test timeout handling
    info!("🧪 Testing timeout handling");

    let timeout_payload = json!({
        "message": "This might timeout",
        "number": 42,
        "test_type": "timeout_test"
    });

    let timeout_request = RICInvokeRequest {
        payload: timeout_payload,
        timeout: Some(1), // Very short timeout
    };

    // This might timeout depending on container startup time
    match executor.invoke_function(&function_id, &timeout_request).await {
        Ok(response) => {
            info!("✅ Function completed within timeout: {:?}", response);
            assert_eq!(response.status_code, 200);
        }
        Err(e) => {
            info!("✅ Function timed out as expected: {}", e);
            assert!(e.to_string().contains("timeout") || e.to_string().contains("Timeout"));
        }
    }

    info!("🎉 Real-World Lambda Error Handling Test completed successfully!");
}
