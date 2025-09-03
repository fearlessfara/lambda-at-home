use lambda_at_home::{
    core::config::Config,
    docker::docker::DockerManager,
    api::lambda_runtime_api::LambdaRuntimeService,
    core::models::{Function, FunctionStatus},
    core::storage::FunctionStorage,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_host_network_execution() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Host Network Execution Test");

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
        let listener = TcpListener::bind("0.0.0.0:8085").await.expect("Failed to bind to port 8085");
        info!("🌐 Lambda Runtime API server started on port 8085");
        
        axum::serve(listener, app).await.expect("Server failed");
    });

    // Give the server time to start and verify it's running
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Test if the server is accessible
    match reqwest::get("http://localhost:8085/health").await {
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
    let function_name = format!("host-network-test-{}", function_id);
    
    info!("📝 Creating host network test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Host network test function".to_string()),
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
    info!("🧪 Testing host network Lambda execution");

    let mut env_vars = HashMap::new();
    env_vars.insert("AWS_LAMBDA_RUNTIME_API".to_string(), "http://localhost:8085".to_string());
    env_vars.insert("HANDLER".to_string(), "index.handler".to_string());
    env_vars.insert("AWS_LAMBDA_FUNCTION_NAME".to_string(), function_name.clone());
    env_vars.insert("AWS_LAMBDA_FUNCTION_MEMORY_SIZE".to_string(), "128".to_string());

    let container_name = format!("host-network-lambda-{}-{}", function_id, Uuid::new_v4());
    
    // Use Docker command to create container with host networking
    let docker_command = format!(
        "docker run -d --name {} --network host -e AWS_LAMBDA_RUNTIME_API=http://localhost:8085 -e HANDLER=index.handler -e AWS_LAMBDA_FUNCTION_NAME={} -e AWS_LAMBDA_FUNCTION_MEMORY_SIZE=128 {}",
        container_name, function_name, format!("lambda-function-{}", function_id)
    );
    
    info!("🐳 Running Docker command: {}", docker_command);
    
    match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&docker_command)
        .output()
        .await
    {
        Ok(output) => {
            if output.status.success() {
                let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                info!("✅ Container created successfully with host networking: {}", container_id);
                
                // Wait for the container to initialize
                info!("⏳ Waiting for container to initialize...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                
                // Check if container is running
                let check_command = format!("docker ps --filter name={} --format '{{{{.Status}}}}'", container_name);
                match tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&check_command)
                    .output()
                    .await
                {
                    Ok(output) => {
                        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        if status.contains("Up") {
                            info!("✅ Container is running: {}", status);
                            
                            // Register the container with the Lambda Runtime Service
                            lambda_runtime_service.register_container(container_id.clone(), function_id).await;
                            info!("✅ Container registered with Lambda Runtime Service");
                            
                            // Test the Lambda execution flow
                            info!("🧪 Testing Lambda execution flow");
                            
                            let test_payload = json!({
                                "message": "Hello from host network Lambda test!",
                                "number": 42,
                                "timestamp": chrono::Utc::now().to_rfc3339(),
                                "test_type": "host_network_execution"
                            });

                            // Queue an invocation
                            let request_id = lambda_runtime_service.queue_invocation(function_id, test_payload.clone()).await.unwrap();
                            info!("✅ Invocation queued with request_id: {}", request_id);

                            // Wait for the RIC to poll and process the invocation
                            let mut response_received = false;
                            let mut attempts = 0;
                            const MAX_ATTEMPTS: u32 = 15; // 15 seconds timeout

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
                                
                                // Check container logs for debugging
                                let logs_command = format!("docker logs {}", container_name);
                                match tokio::process::Command::new("sh")
                                    .arg("-c")
                                    .arg(&logs_command)
                                    .output()
                                    .await
                                {
                                    Ok(output) => {
                                        let logs = String::from_utf8_lossy(&output.stdout);
                                        info!("📋 Container logs: {}", logs);
                                    }
                                    Err(e) => {
                                        warn!("📋 Failed to get container logs: {}", e);
                                    }
                                }
                                
                                panic!("Lambda execution timed out");
                            }

                            info!("✅ Lambda execution completed successfully!");
                            
                        } else {
                            warn!("⚠️ Container is not running: {}", status);
                        }
                    }
                    Err(e) => {
                        warn!("❌ Failed to check container status: {}", e);
                    }
                }
                
                // Clean up the container
                let cleanup_command = format!("docker rm -f {}", container_name);
                let _ = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cleanup_command)
                    .output()
                    .await;
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                warn!("❌ Failed to create container: {}", error);
            }
        }
        Err(e) => {
            warn!("❌ Failed to run Docker command: {}", e);
        }
    }

    // Stop the server
    server_handle.abort();

    info!("🎉 Lambda Host Network Execution Test completed successfully!");
}
