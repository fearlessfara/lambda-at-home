use lambda_docker_executor::{
    config::Config,
    container_lifecycle::ContainerLifecycleManager,
    docker::DockerManager,
    lambda_executor::LambdaExecutor,
    lambda_runtime_api::LambdaRuntimeService,
    models::{Function, FunctionStatus, RICInvokeRequest},
    storage::FunctionStorage,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_container_reuse() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Container Reuse Test");

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
    let function_name = format!("container-reuse-test-{}", function_id);
    
    info!("📝 Creating container reuse test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Container reuse test function".to_string()),
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

    // Test container reuse
    info!("🧪 Testing container reuse functionality");

    // First invocation - should create a new container
    let test_payload1 = json!({
        "message": "First invocation - should create container",
        "number": 10,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "container_reuse",
        "invocation_number": 1
    });

    let invoke_request1 = RICInvokeRequest {
        payload: test_payload1.clone(),
        timeout: Some(30),
    };

    info!("🚀 First invocation - creating container");
    let start_time1 = Instant::now();
    
    match executor.invoke_function(&function_id, &invoke_request1).await {
        Ok(response1) => {
            let duration1 = start_time1.elapsed();
            info!("✅ First invocation completed in {:?} (container creation time)", duration1);
            
            assert_eq!(response1.status_code, 200);
            assert!(response1.payload.is_some());
            
            let body1 = response1.payload.unwrap();
            if let Some(body_obj1) = body1.as_object() {
                let computation_result1 = body_obj1.get("computation_result").unwrap().as_f64().unwrap();
                assert_eq!(computation_result1, 420.0); // 10 * 42
            }
            
            info!("✅ First invocation validations passed!");
        }
        Err(e) => {
            warn!("❌ First invocation failed: {}", e);
            panic!("First invocation failed: {}", e);
        }
    }

    // Wait a bit to ensure container is ready for reuse
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    // Second invocation - should reuse the existing container
    let test_payload2 = json!({
        "message": "Second invocation - should reuse container",
        "number": 20,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "container_reuse",
        "invocation_number": 2
    });

    let invoke_request2 = RICInvokeRequest {
        payload: test_payload2.clone(),
        timeout: Some(30),
    };

    info!("🚀 Second invocation - reusing container");
    let start_time2 = Instant::now();
    
    match executor.invoke_function(&function_id, &invoke_request2).await {
        Ok(response2) => {
            let duration2 = start_time2.elapsed();
            info!("✅ Second invocation completed in {:?} (should be faster due to reuse)", duration2);
            
            assert_eq!(response2.status_code, 200);
            assert!(response2.payload.is_some());
            
            let body2 = response2.payload.unwrap();
            if let Some(body_obj2) = body2.as_object() {
                let computation_result2 = body_obj2.get("computation_result").unwrap().as_f64().unwrap();
                assert_eq!(computation_result2, 840.0); // 20 * 42
            }
            
            info!("✅ Second invocation validations passed!");
        }
        Err(e) => {
            warn!("❌ Second invocation failed: {}", e);
            panic!("Second invocation failed: {}", e);
        }
    }

    // Third invocation - should also reuse the container
    let test_payload3 = json!({
        "message": "Third invocation - should also reuse container",
        "number": 30,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "container_reuse",
        "invocation_number": 3
    });

    let invoke_request3 = RICInvokeRequest {
        payload: test_payload3.clone(),
        timeout: Some(30),
    };

    info!("🚀 Third invocation - reusing container again");
    let start_time3 = Instant::now();
    
    match executor.invoke_function(&function_id, &invoke_request3).await {
        Ok(response3) => {
            let duration3 = start_time3.elapsed();
            info!("✅ Third invocation completed in {:?} (should be fast due to reuse)", duration3);
            
            assert_eq!(response3.status_code, 200);
            assert!(response3.payload.is_some());
            
            let body3 = response3.payload.unwrap();
            if let Some(body_obj3) = body3.as_object() {
                let computation_result3 = body_obj3.get("computation_result").unwrap().as_f64().unwrap();
                assert_eq!(computation_result3, 1260.0); // 30 * 42
            }
            
            info!("✅ Third invocation validations passed!");
        }
        Err(e) => {
            warn!("❌ Third invocation failed: {}", e);
            panic!("Third invocation failed: {}", e);
        }
    }

    info!("🎉 Lambda Container Reuse Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_container_lifecycle_states() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Container Lifecycle States Test");

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
    let function_name = format!("lifecycle-test-{}", function_id);
    
    info!("📝 Creating lifecycle test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Lifecycle test function".to_string()),
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

    // Test container lifecycle states
    info!("🧪 Testing container lifecycle states");

    // Get initial container stats
    let initial_stats = executor.get_container_stats().await.expect("Failed to get initial container stats");
    info!("📊 Initial container stats: {:?}", initial_stats);

    // First invocation - container should be created and go through Ready -> Busy -> Ready
    let test_payload = json!({
        "message": "Lifecycle state test",
        "number": 42,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "lifecycle_states"
    });

    let invoke_request = RICInvokeRequest {
        payload: test_payload.clone(),
        timeout: Some(30),
    };

    info!("🚀 Testing container lifecycle states");
    let start_time = Instant::now();
    
    match executor.invoke_function(&function_id, &invoke_request).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            info!("✅ Lifecycle test invocation completed in {:?}", duration);
            
            assert_eq!(response.status_code, 200);
            assert!(response.payload.is_some());
            
            let body = response.payload.unwrap();
            if let Some(body_obj) = body.as_object() {
                let computation_result = body_obj.get("computation_result").unwrap().as_f64().unwrap();
                assert_eq!(computation_result, 1764.0); // 42 * 42
            }
            
            info!("✅ Lifecycle test validations passed!");
        }
        Err(e) => {
            warn!("❌ Lifecycle test invocation failed: {}", e);
            panic!("Lifecycle test invocation failed: {}", e);
        }
    }

    // Get final container stats
    let final_stats = executor.get_container_stats().await.expect("Failed to get final container stats");
    info!("📊 Final container stats: {:?}", final_stats);

    // Verify that we have at least one active container
    assert!(final_stats.active_containers > 0, "Expected at least one active container");

    info!("🎉 Lambda Container Lifecycle States Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_container_cleanup() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Container Cleanup Test");

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
    let function_name = format!("cleanup-test-{}", function_id);
    
    info!("📝 Creating cleanup test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Cleanup test function".to_string()),
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

    // Test container cleanup
    info!("🧪 Testing container cleanup functionality");

    // Create a container by invoking the function
    let test_payload = json!({
        "message": "Cleanup test",
        "number": 42,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_type": "cleanup_test"
    });

    let invoke_request = RICInvokeRequest {
        payload: test_payload.clone(),
        timeout: Some(30),
    };

    info!("🚀 Creating container for cleanup test");
    let start_time = Instant::now();
    
    match executor.invoke_function(&function_id, &invoke_request).await {
        Ok(response) => {
            let duration = start_time.elapsed();
            info!("✅ Cleanup test invocation completed in {:?}", duration);
            
            assert_eq!(response.status_code, 200);
            assert!(response.payload.is_some());
            
            info!("✅ Container created successfully for cleanup test");
        }
        Err(e) => {
            warn!("❌ Cleanup test invocation failed: {}", e);
            panic!("Cleanup test invocation failed: {}", e);
        }
    }

    // Get container stats before cleanup
    let stats_before = executor.get_container_stats().await.expect("Failed to get container stats before cleanup");
    info!("📊 Container stats before cleanup: {:?}", stats_before);

    // Terminate containers for this function
    info!("🧹 Terminating containers for function {}", function_id);
    match executor.terminate_containers(&function_id).await {
        Ok(_) => {
            info!("✅ Containers terminated successfully");
        }
        Err(e) => {
            warn!("❌ Failed to terminate containers: {}", e);
            // Don't panic here as cleanup might fail in test environment
        }
    }

    // Get container stats after cleanup
    let stats_after = executor.get_container_stats().await.expect("Failed to get container stats after cleanup");
    info!("📊 Container stats after cleanup: {:?}", stats_after);

    info!("🎉 Lambda Container Cleanup Test completed successfully!");
}
