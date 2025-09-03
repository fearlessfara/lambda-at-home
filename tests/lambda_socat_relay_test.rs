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
async fn test_lambda_socat_relay_execution() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Socat Relay Execution Test");

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
        let listener = TcpListener::bind("0.0.0.0:8086").await.expect("Failed to bind to port 8086");
        info!("🌐 Lambda Runtime API server started on port 8086");
        
        axum::serve(listener, app).await.expect("Server failed");
    });

    // Give the server time to start and verify it's running
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    
    // Test if the server is accessible
    match reqwest::get("http://localhost:8086/health").await {
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
    let function_name = format!("socat-relay-test-{}", function_id);
    
    info!("📝 Creating socat relay test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Socat relay test function".to_string()),
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

    // Create Docker network for the relay
    info!("🌐 Creating Docker network for socat relay");
    let network_name = format!("lambda-net-{}", Uuid::new_v4());
    let create_network_cmd = format!("docker network create {}", network_name);
    
    let _ = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&create_network_cmd)
        .output()
        .await;

    // Start socat relay container
    info!("🔄 Starting socat relay container");
    let relay_name = format!("host-relay-{}", Uuid::new_v4());
    let relay_cmd = format!(
        "docker run -d --name {} --network {} alpine/socat -d -d TCP-LISTEN:8086,fork,reuseaddr TCP:host.docker.internal:8086",
        relay_name, network_name
    );
    
    let relay_output = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&relay_cmd)
        .output()
        .await;

    match relay_output {
        Ok(output) => {
            if output.status.success() {
                let relay_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                info!("✅ Socat relay started: {}", relay_id);
                
                // Wait for relay to start
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                
                // Start Lambda container on the same network
                info!("🐳 Starting Lambda container with socat relay");
                let container_name = format!("socat-lambda-{}-{}", function_id, Uuid::new_v4());
                let lambda_cmd = format!(
                    "docker run -d --name {} --network {} -e AWS_LAMBDA_RUNTIME_API=http://{}:8086 -e HANDLER=index.handler -e AWS_LAMBDA_FUNCTION_NAME={} -e AWS_LAMBDA_FUNCTION_MEMORY_SIZE=128 {}",
                    container_name, network_name, relay_name, function_name, format!("lambda-function-{}", function_id)
                );
                
                let lambda_output = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(&lambda_cmd)
                    .output()
                    .await;

                match lambda_output {
                    Ok(output) => {
                        if output.status.success() {
                            let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            info!("✅ Lambda container started: {}", container_id);
                            
                            // Wait for the container to initialize
                            info!("⏳ Waiting for container to initialize...");
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                            
                            // Check if container is running
                            let check_command = format!("docker ps --filter name={} --format '{{{{.Status}}}}'", container_name);
                            let status_output = tokio::process::Command::new("sh")
                                .arg("-c")
                                .arg(&check_command)
                                .output()
                                .await;

                            match status_output {
                                Ok(output) => {
                                    let status = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                    if status.contains("Up") {
                                        info!("✅ Container is running: {}", status);
                                        
                                        // Register the container with the Lambda Runtime Service
                                        lambda_runtime_service.register_container(container_id.clone(), function_id).await;
                                        info!("✅ Container registered with Lambda Runtime Service");
                                        
                                        // Test the Lambda execution flow
                                        info!("🧪 Testing Lambda execution flow with socat relay");
                                        
                                        let test_payload = json!({
                                            "message": "Hello from socat relay Lambda test!",
                                            "number": 42,
                                            "timestamp": chrono::Utc::now().to_rfc3339(),
                                            "test_type": "socat_relay_execution"
                                        });

                                        // Queue an invocation
                                        let request_id = lambda_runtime_service.queue_invocation(function_id, test_payload.clone()).await.unwrap();
                                        info!("✅ Invocation queued with request_id: {}", request_id);

                                        // Wait for the RIC to poll and process the invocation
                                        let mut response_received = false;
                                        let mut attempts = 0;
                                        const MAX_ATTEMPTS: u32 = 20; // 20 seconds timeout

                                        while !response_received && attempts < MAX_ATTEMPTS {
                                            // Check for completed response
                                            if let Ok(Some(response)) = lambda_runtime_service.get_response(&request_id).await {
                                                info!("✅ Response received: {:?}", response);
                                                
                                                // Verify response structure
                                                let body = response.response;
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
                                                    assert_eq!(received_payload.get("test_type").unwrap().as_str().unwrap(), "socat_relay_execution");
                                                }
                                                
                                                response_received = true;
                                                info!("✅ All response validations passed!");
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
                                            if let Ok(logs_output) = tokio::process::Command::new("sh")
                                                .arg("-c")
                                                .arg(&logs_command)
                                                .output()
                                                .await
                                            {
                                                let logs = String::from_utf8_lossy(&logs_output.stdout);
                                                info!("📋 Container logs: {}", logs);
                                            }
                                            
                                            panic!("Lambda execution timed out");
                                        }

                                        info!("✅ Lambda execution completed successfully with socat relay!");
                                        
                                    } else {
                                        warn!("⚠️ Container is not running: {}", status);
                                    }
                                }
                                Err(e) => {
                                    warn!("❌ Failed to check container status: {}", e);
                                }
                            }
                            
                        } else {
                            let error = String::from_utf8_lossy(&output.stderr);
                            warn!("❌ Failed to start Lambda container: {}", error);
                        }
                    }
                    Err(e) => {
                        warn!("❌ Failed to run Lambda container command: {}", e);
                    }
                }
                
                // Clean up containers and network
                let cleanup_commands = vec![
                    format!("docker rm -f {}", container_name),
                    format!("docker rm -f {}", relay_name),
                    format!("docker network rm {}", network_name),
                ];
                
                for cmd in cleanup_commands {
                    let _ = tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg(&cmd)
                        .output()
                        .await;
                }
                
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                warn!("❌ Failed to start socat relay: {}", error);
            }
        }
        Err(e) => {
            warn!("❌ Failed to run socat relay command: {}", e);
        }
    }

    // Stop the server
    server_handle.abort();

    info!("🎉 Lambda Socat Relay Execution Test completed successfully!");
}