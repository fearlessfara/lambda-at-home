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

use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_full_integration_flow() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Full Integration Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::with_max_concurrency(5));
    let lifecycle_manager = Arc::new(ContainerLifecycleManager::new(
        docker_manager.clone(),
        config.clone(),
        storage.clone(),
    ));

    // Create Lambda executor
    let executor = LambdaExecutor::new(
        docker_manager,
        lifecycle_manager.clone(),
        lambda_runtime_service.clone(),
    );

    // Create a test function
    let function_id = Uuid::new_v4();
    let function_name = format!("test-function-{}", function_id);
    
    info!("📝 Creating test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Test function for Lambda full integration".to_string()),
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
    info!("🧪 Testing concurrent Lambda invocations");

    let mut handles = Vec::new();
    let num_concurrent = 3;

    for i in 0..num_concurrent {
        let executor_clone = executor.clone();
        let function_id_clone = function_id;
        
        let handle = tokio::spawn(async move {
            let test_payload = json!({
                "message": format!("Hello from concurrent invocation {}!", i),
                "number": i * 10,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });

            let invoke_request = RICInvokeRequest {
                payload: test_payload.clone(),
                timeout: Some(30),
            };

            info!("🚀 Starting concurrent invocation {}", i);
            let start_time = std::time::Instant::now();
            
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

    // Test queueing behavior
    info!("🧪 Testing Lambda queueing behavior");

    // Create a service with low concurrency limit
    let limited_service = Arc::new(LambdaRuntimeService::with_max_concurrency(1));
    let limited_executor = LambdaExecutor::new(
        DockerManager::new().await.expect("Failed to create Docker manager"),
        lifecycle_manager.clone(),
        limited_service.clone(),
    );

    // Queue multiple invocations rapidly
    let mut queue_handles = Vec::new();
    for i in 0..3 {
        let executor_clone = limited_executor.clone();
        let function_id_clone = function_id;
        
        let handle = tokio::spawn(async move {
            let test_payload = json!({
                "message": format!("Queued invocation {}", i),
                "number": i,
            });

            let invoke_request = RICInvokeRequest {
                payload: test_payload,
                timeout: Some(30),
            };

            info!("🚀 Starting queued invocation {}", i);
            let start_time = std::time::Instant::now();
            
            match executor_clone.invoke_function(&function_id_clone, &invoke_request).await {
                Ok(response) => {
                    let duration = start_time.elapsed();
                    info!("✅ Queued invocation {} completed in {:?}", i, duration);
                    assert_eq!(response.status_code, 200);
                    Ok(())
                }
                Err(e) => {
                    warn!("❌ Queued invocation {} failed: {}", i, e);
                    Err(e)
                }
            }
        });
        
        queue_handles.push(handle);
    }

    // Wait for all queued invocations to complete
    let mut queue_results = Vec::new();
    for (i, handle) in queue_handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(())) => {
                info!("✅ Queued invocation {} completed successfully", i);
                queue_results.push(Ok(()));
            }
            Ok(Err(e)) => {
                warn!("❌ Queued invocation {} failed: {}", i, e);
                queue_results.push(Err(e));
            }
            Err(e) => {
                warn!("❌ Queued invocation {} task failed: {}", i, e);
                queue_results.push(Err(anyhow::anyhow!("Task failed: {}", e)));
            }
        }
    }

    // Verify all queued invocations succeeded
    for (i, result) in queue_results.iter().enumerate() {
        assert!(result.is_ok(), "Queued invocation {} failed: {:?}", i, result);
    }

    info!("🎉 All queued invocations completed successfully!");

    // Test error handling
    info!("🧪 Testing Lambda error handling");

    // This test would require a function that actually fails
    // For now, we'll just test the error handling in the runtime service
    let error_service = Arc::new(LambdaRuntimeService::new());
    let error_function_id = Uuid::new_v4();
    let error_container_id = "error-container".to_string();

    error_service.register_container(error_container_id.clone(), error_function_id).await;

    // Queue invocation
    let error_payload = json!({"test": "error"});
    let error_request_id = error_service.queue_invocation(error_function_id, error_payload).await.unwrap();

    // Retrieve invocation
    let error_invocation = error_service.get_next_invocation(&error_container_id).await.unwrap().unwrap();
    assert_eq!(error_invocation.request_id, error_request_id);

    // Submit error
    error_service.submit_error(
        error_request_id.clone(),
        "TestError".to_string(),
        "This is a test error".to_string(),
        Some(vec!["line 1".to_string()])
    ).await.unwrap();

    // Verify error was received
    let received_error = error_service.get_error(&error_request_id).await.unwrap();
    assert!(received_error.is_some());
    assert_eq!(received_error.unwrap().error_type, "TestError");

    info!("✅ Error handling test passed!");

    info!("🎉 Lambda Full Integration Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_runtime_api_endpoints() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🧪 Testing Lambda Runtime API endpoints");

    let runtime_service = Arc::new(LambdaRuntimeService::new());
    let function_id = Uuid::new_v4();
    let container_id = "test-container-endpoints".to_string();

    // Test container registration
    runtime_service.register_container(container_id.clone(), function_id).await;
    info!("✅ Container registered successfully");

    // Test invocation queuing
    let test_payload = json!({
        "test": "endpoint_test",
        "number": 42
    });

    let request_id = runtime_service.queue_invocation(function_id, test_payload.clone()).await.unwrap();
    info!("✅ Invocation queued with request_id: {}", request_id);

    // Test polling for next invocation (simulating RIC behavior)
    let invocation = runtime_service.get_next_invocation(&container_id).await.unwrap();
    assert!(invocation.is_some());
    let invocation = invocation.unwrap();
    assert_eq!(invocation.request_id, request_id);
    assert_eq!(invocation.function_id, function_id);
    assert_eq!(invocation.payload, test_payload);
    info!("✅ Successfully retrieved invocation via polling");

    // Test response submission
    let response = json!({
        "statusCode": 200,
        "body": {
            "message": "Endpoint test response",
            "computation": 1764.0
        }
    });

    runtime_service.submit_response(request_id.clone(), response.clone()).await.unwrap();
    info!("✅ Response submitted successfully");

    // Test response retrieval
    let retrieved_response = runtime_service.get_response(&request_id).await.unwrap();
    assert!(retrieved_response.is_some());
    assert_eq!(retrieved_response.unwrap().response, response);
    info!("✅ Response retrieved successfully");

    info!("🎉 Lambda Runtime API endpoint tests completed successfully!");
}
