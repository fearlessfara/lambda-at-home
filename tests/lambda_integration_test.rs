use lambda_at_home::{
    core::config::Config,
    docker::container_lifecycle::ContainerLifecycleManager,
    docker::docker::DockerManager,
    api::lambda_executor::LambdaExecutor,
    api::lambda_runtime_api::LambdaRuntimeService,
    core::models::{Function, FunctionStatus, RICInvokeRequest},
    core::storage::FunctionStorage,
};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_integration_flow() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Integration Test");

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
        config,
        storage.clone(),
        lifecycle_manager,
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
        description: Some("Test function for Lambda integration".to_string()),
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

    // Create test payload
    let test_payload = serde_json::json!({
        "message": "Hello from Lambda integration test!",
        "number": 42,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    let invoke_request = RICInvokeRequest {
        payload: test_payload.clone(),
        timeout: Some(30),
    };

    info!("🧪 Testing Lambda invocation flow");

    // Test the complete flow
    let start_time = std::time::Instant::now();
    
    match executor.invoke_function(&function_id, &invoke_request).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            info!("✅ Lambda invocation completed successfully in {:?}", duration);
            info!("📊 Response: {:?}", response);
            
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
                
                // Verify computation result (42 * 42 = 1764)
                let computation_result = body_obj.get("computation_result").unwrap().as_f64().unwrap();
                assert_eq!(computation_result, 1764.0);
            } else {
                panic!("Expected response body to be an object");
            }
            
            info!("✅ All response validations passed!");
        }
        Err(e) => {
            warn!("❌ Lambda invocation failed: {}", e);
            panic!("Lambda invocation failed: {}", e);
        }
    }

    info!("🎉 Lambda Integration Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_runtime_service_endpoints() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🧪 Testing Lambda Runtime Service endpoints");

    let runtime_service = Arc::new(LambdaRuntimeService::new());
    let function_id = Uuid::new_v4();
    let container_id = "test-container-123".to_string();

    // Test container registration
    runtime_service.register_container(container_id.clone(), function_id).await;
    info!("✅ Container registered successfully");

    // Test invocation queuing
    let test_payload = serde_json::json!({
        "test": "payload",
        "number": 10
    });

    let request_id = runtime_service.queue_invocation(function_id, test_payload.clone()).await.unwrap();
    info!("✅ Invocation queued with request_id: {}", request_id);

    // Test polling for next invocation (simulating RIC behavior)
    let invocation = runtime_service.get_next_invocation(&container_id).await.unwrap();
    assert_eq!(invocation.request_id, request_id);
    assert_eq!(invocation.function_id, function_id);
    assert_eq!(invocation.payload, test_payload);
    info!("✅ Successfully retrieved invocation via polling");

    // Test response submission
    let response = serde_json::json!({
        "statusCode": 200,
        "body": {
            "message": "Test response",
            "computation": 420.0
        }
    });

    runtime_service.submit_response(request_id.clone(), response.clone()).await.unwrap();
    info!("✅ Response submitted successfully");

    // Test response retrieval
    let retrieved_response = runtime_service.get_response(&request_id).await.unwrap();
    assert_eq!(retrieved_response.response, response);
    info!("✅ Response retrieved successfully");

    info!("🎉 Lambda Runtime Service endpoint tests completed successfully!");
}
