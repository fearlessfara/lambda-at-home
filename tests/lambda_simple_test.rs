use lambda_docker_executor::{
    config::Config,
    docker::DockerManager,
    lambda_runtime_api::LambdaRuntimeService,
    models::{Function, FunctionStatus},
    storage::FunctionStorage,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_simple_execution() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Simple Execution Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::new());

    // Start the Lambda Runtime API server
    let server_service = lambda_runtime_service.clone();
    let server_handle = tokio::spawn(async move {
        let app = server_service.create_router().with_state(server_service);
        let listener = TcpListener::bind("0.0.0.0:8083").await.expect("Failed to bind to port 8083");
        info!("🌐 Lambda Runtime API server started on port 8083");
        
        axum::serve(listener, app).await.expect("Server failed");
    });

    // Give the server time to start and verify it's running
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Test if the server is accessible
    match reqwest::get("http://localhost:8083/health").await {
        Ok(response) => {
            info!("✅ Lambda Runtime API server is accessible: {}", response.status());
        }
        Err(e) => {
            warn!("❌ Lambda Runtime API server is not accessible: {}", e);
            panic!("Server is not accessible: {}", e);
        }
    }

    // Create a test function
    let function_id = Uuid::parse_str("ad80c2e0-aedc-4809-a27b-317361ec87e6").unwrap();
    let function_name = format!("simple-test-{}", function_id);
    
    info!("📝 Creating simple test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Simple test function".to_string()),
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

    // Manually create a container with proper environment variables
    info!("🧪 Testing simple Lambda execution with proper environment");

    let mut env_vars = HashMap::new();
    env_vars.insert("AWS_LAMBDA_RUNTIME_API".to_string(), "http://host.docker.internal:8083".to_string());
    env_vars.insert("HANDLER".to_string(), "index.handler".to_string());
    env_vars.insert("AWS_LAMBDA_FUNCTION_NAME".to_string(), function_name.clone());
    env_vars.insert("AWS_LAMBDA_FUNCTION_MEMORY_SIZE".to_string(), "128".to_string());

    let container_name = format!("simple-lambda-{}-{}", function_id, Uuid::new_v4());
    
    match docker_manager.create_container(
        &function_id,
        &container_name,
        &env_vars,
        128,
        0.5,
    ).await {
        Ok(container_id) => {
            info!("✅ Container created successfully: {}", container_id);
            
            // Start the container
            match docker_manager.start_container(&container_id).await {
                Ok(_) => {
                    info!("✅ Container started successfully");
                    
                    // Register the container with the Lambda Runtime Service
                    lambda_runtime_service.register_container(container_id.clone(), function_id).await;
                    info!("✅ Container registered with Lambda Runtime Service");
                    
                    // Wait for the container to initialize
                    info!("⏳ Waiting for container to initialize...");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    
                    // Check if container is running
                    match docker_manager.is_container_running(&container_id).await {
                        Ok(is_running) => {
                            if is_running {
                                info!("✅ Container is running");
                                
                                // Test the Lambda execution flow
                                info!("🧪 Testing Lambda execution flow");
                                
                                let test_payload = json!({
                                    "message": "Hello from simple Lambda test!",
                                    "number": 42,
                                    "timestamp": chrono::Utc::now().to_rfc3339(),
                                    "test_type": "simple_execution"
                                });

                                // Queue an invocation
                                let request_id = lambda_runtime_service.queue_invocation(function_id, test_payload.clone()).await.unwrap();
                                info!("✅ Invocation queued with request_id: {}", request_id);

                                // Wait for the RIC to poll and process the invocation
                                let mut response_received = false;
                                let mut attempts = 0;
                                const MAX_ATTEMPTS: u32 = 10; // 10 seconds timeout

                                while !response_received && attempts < MAX_ATTEMPTS {
                                    // Check for completed response
                                    if let Ok(Some(response)) = lambda_runtime_service.get_response(&request_id).await {
                                        info!("✅ Response received: {:?}", response);
                                        response_received = true;
                                    } else if let Ok(Some(error)) = lambda_runtime_service.get_error(&request_id).await {
                                        warn!("❌ Error received: {:?}", error);
                                        panic!("Lambda execution failed with error: {}", error.error_message);
                                    } else {
                                        // No response yet, wait a bit
                                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                                        attempts += 1;
                                    }
                                }

                                if !response_received {
                                    warn!("❌ Timeout waiting for response after {} attempts", MAX_ATTEMPTS);
                                    panic!("Lambda execution timed out");
                                }

                                info!("✅ Lambda execution completed successfully!");
                                
                            } else {
                                warn!("⚠️ Container is not running");
                            }
                        }
                        Err(e) => {
                            warn!("❌ Failed to check container status: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("❌ Failed to start container: {}", e);
                }
            }
        }
        Err(e) => {
            warn!("❌ Failed to create container: {}", e);
        }
    }

    // Stop the server
    server_handle.abort();

    info!("🎉 Lambda Simple Execution Test completed successfully!");
}
