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
use tokio::net::TcpListener;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_full_flow_with_server() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Full Flow Test with Server");

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

    // Start the Lambda Runtime API server
    let server_service = lambda_runtime_service.clone();
    let server_handle = tokio::spawn(async move {
        let app = server_service.create_router().with_state(server_service);
        let listener = TcpListener::bind("0.0.0.0:8080").await.expect("Failed to bind to port 8080");
        info!("🌐 Lambda Runtime API server started on port 8080");
        
        axum::serve(listener, app).await.expect("Server failed");
    });

    // Give the server time to start
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Create Lambda executor
    let executor = LambdaExecutor::new(
        docker_manager,
        lifecycle_manager,
        lambda_runtime_service.clone(),
    );

    // Create a test function using an existing function ID that has a Docker image
    let function_id = Uuid::parse_str("ad80c2e0-aedc-4809-a27b-317361ec87e6").unwrap();
    let function_name = format!("full-flow-test-{}", function_id);
    
    info!("📝 Creating full flow test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Full flow test function".to_string()),
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

    // Test the full flow
    info!("🧪 Testing full Lambda flow with server");

    let test_payload = json!({
        "message": "Hello from full flow test!",
        "number": 42,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "full_flow_with_server"
    });

    let invoke_request = RICInvokeRequest {
        payload: test_payload.clone(),
        timeout: Some(30),
    };

    let start_time = Instant::now();
    
    match executor.invoke_function(&function_id, &invoke_request).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            info!("✅ Full flow test completed successfully in {:?}", duration);
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
                assert_eq!(received_payload.get("test_type").unwrap().as_str().unwrap(), "full_flow_with_server");
            } else {
                panic!("Expected response body to be an object");
            }
            
            info!("✅ All full flow response validations passed!");
        }
        Err(e) => {
            warn!("❌ Full flow test failed: {}", e);
            panic!("Full flow test failed: {}", e);
        }
    }

    // Stop the server
    server_handle.abort();

    info!("🎉 Lambda Full Flow Test with Server completed successfully!");
}
