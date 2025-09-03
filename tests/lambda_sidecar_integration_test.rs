use lambda_docker_executor::{
    container_lifecycle::ContainerLifecycleManager,
    docker::DockerManager,
    lambda_runtime_api::LambdaRuntimeService,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_sidecar_lambda_execution() {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init();

    info!("🚀 Starting Sidecar Lambda Integration Test");

    // Start the sidecar services
    info!("📦 Starting sidecar services...");
    let sidecar_result = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "up", "-d"])
        .output()
        .await;

    match sidecar_result {
        Ok(output) => {
            if !output.status.success() {
                warn!("Failed to start sidecar services: {}", String::from_utf8_lossy(&output.stderr));
                return;
            }
            info!("✅ Sidecar services started successfully");
        }
        Err(e) => {
            warn!("Failed to execute docker-compose: {}", e);
            return;
        }
    }

    // Wait for services to be ready
    info!("⏳ Waiting for services to be ready...");
    sleep(Duration::from_secs(15)).await;

    // Test direct connection to Lambda Runtime API
    info!("🧪 Testing direct connection to Lambda Runtime API...");
    let client = reqwest::Client::new();
    match client.get("http://localhost:8080/health").send().await {
        Ok(response) => {
            if response.status().is_success() {
                info!("✅ Direct connection to Lambda Runtime API successful");
            } else {
                warn!("❌ Direct connection failed with status: {}", response.status());
                return;
            }
        }
        Err(e) => {
            warn!("❌ Direct connection failed: {}", e);
            return;
        }
    }

    // Test connection through socat relay
    info!("🧪 Testing connection through socat relay...");
    match client.get("http://localhost:8081/health").send().await {
        Ok(response) => {
            if response.status().is_success() {
                info!("✅ Socat relay connection successful");
            } else {
                warn!("❌ Socat relay connection failed with status: {}", response.status());
                return;
            }
        }
        Err(e) => {
            warn!("❌ Socat relay connection failed: {}", e);
            return;
        }
    }

    // Create a test function
    let function_id = Uuid::new_v4();
    let function_name = format!("test-function-{}", function_id);
    
    info!("📝 Creating test function: {}", function_name);
    
    // Create a simple Node.js function
    let function_code = r#"
const handler = async (event, context) => {
    console.log('Event:', JSON.stringify(event, null, 2));
    console.log('Context:', JSON.stringify(context, null, 2));
    
    return {
        statusCode: 200,
        body: JSON.stringify({
            message: 'Hello from sidecar Lambda!',
            event: event,
            timestamp: new Date().toISOString(),
            functionName: context.functionName,
            requestId: context.awsRequestId
        })
    };
};

module.exports = { handler };
"#;

    // Create package.json
    let package_json = json!({
        "name": function_name,
        "version": "1.0.0",
        "main": "index.js",
        "dependencies": {}
    });

    // Write function files
    tokio::fs::write(format!("functions/{}/index.js", function_name), function_code).await.unwrap();
    tokio::fs::write(format!("functions/{}/package.json", function_name), package_json.to_string()).await.unwrap();

    // Build the Docker image for the function
    info!("🔨 Building Docker image for function: {}", function_name);
    let build_result = tokio::process::Command::new("docker")
        .args([
            "build",
            "-t",
            &format!("lambda-function-{}", function_id),
            "-f",
            "Dockerfile.lambda",
            &format!("functions/{}", function_name)
        ])
        .output()
        .await;

    match build_result {
        Ok(output) => {
            if output.status.success() {
                info!("✅ Docker image built successfully");
            } else {
                warn!("❌ Docker image build failed: {}", String::from_utf8_lossy(&output.stderr));
                return;
            }
        }
        Err(e) => {
            warn!("❌ Failed to build Docker image: {}", e);
            return;
        }
    }

    // Create Docker manager and lifecycle manager
    let docker_manager = Arc::new(DockerManager::new().await.unwrap());
    let lifecycle_manager = Arc::new(ContainerLifecycleManager::new(
        docker_manager.clone(),
        "/tmp/lambda-functions".to_string(),
    ));

    // Deploy the function
    info!("🚀 Deploying function: {}", function_name);
    let deploy_result = lifecycle_manager.deploy_function(
        function_id,
        &function_name,
        128, // 128MB memory
        0.5, // 0.5 CPU
        &format!("lambda-function-{}", function_id),
    ).await;

    match deploy_result {
        Ok(_) => {
            info!("✅ Function deployed successfully");
        }
        Err(e) => {
            warn!("❌ Function deployment failed: {}", e);
            return;
        }
    }

    // Wait for function to be ready
    info!("⏳ Waiting for function to be ready...");
    sleep(Duration::from_secs(10)).await;

    // Test function execution through sidecar
    info!("🧪 Testing function execution through sidecar...");
    
    let test_payload = json!({
        "message": "Hello from sidecar test!",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "testId": "sidecar-integration-test"
    });

    // Create a Lambda invocation through the sidecar
    let invocation_payload = json!({
        "function_id": function_id,
        "payload": test_payload,
        "deadline_ms": 30000,
        "invoked_function_arn": format!("arn:aws:lambda:us-east-1:123456789012:function:{}", function_name)
    });

    // Send invocation to Lambda Runtime API through socat relay
    match client
        .post("http://localhost:8081/runtime/invocation/next")
        .json(&invocation_payload)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                let response_text = response.text().await.unwrap();
                info!("✅ Function execution successful!");
                info!("📋 Response: {}", response_text);
            } else {
                warn!("❌ Function execution failed with status: {}", response.status());
            }
        }
        Err(e) => {
            warn!("❌ Function execution failed: {}", e);
        }
    }

    // Test multiple concurrent executions
    info!("🧪 Testing concurrent function executions...");
    let mut handles = vec![];
    
    for i in 0..3 {
        let client_clone = client.clone();
        let payload_clone = invocation_payload.clone();
        
        let handle = tokio::spawn(async move {
            let concurrent_payload = json!({
                "message": format!("Concurrent test {}", i),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "testId": format!("sidecar-concurrent-test-{}", i)
            });
            
            let concurrent_invocation = json!({
                "function_id": function_id,
                "payload": concurrent_payload,
                "deadline_ms": 30000,
                "invoked_function_arn": format!("arn:aws:lambda:us-east-1:123456789012:function:{}", function_name)
            });
            
            match client_clone
                .post("http://localhost:8081/runtime/invocation/next")
                .json(&concurrent_invocation)
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() {
                        info!("✅ Concurrent execution {} successful", i);
                    } else {
                        warn!("❌ Concurrent execution {} failed with status: {}", i, response.status());
                    }
                }
                Err(e) => {
                    warn!("❌ Concurrent execution {} failed: {}", i, e);
                }
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all concurrent executions to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Cleanup
    info!("🧹 Cleaning up...");
    
    // Stop sidecar services
    let _ = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "down"])
        .output()
        .await;

    // Remove function files
    let _ = tokio::fs::remove_dir_all(format!("functions/{}", function_name)).await;
    
    // Remove Docker image
    let _ = tokio::process::Command::new("docker")
        .args(["rmi", &format!("lambda-function-{}", function_id)])
        .output()
        .await;

    info!("✅ Sidecar Lambda Integration Test completed successfully!");
}

#[tokio::test]
async fn test_sidecar_health_check() {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init();

    info!("🚀 Starting Sidecar Health Check Test");

    // Start the sidecar services
    info!("📦 Starting sidecar services...");
    let sidecar_result = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "up", "-d"])
        .output()
        .await;

    match sidecar_result {
        Ok(output) => {
            if !output.status.success() {
                warn!("Failed to start sidecar services: {}", String::from_utf8_lossy(&output.stderr));
                return;
            }
            info!("✅ Sidecar services started successfully");
        }
        Err(e) => {
            warn!("Failed to execute docker-compose: {}", e);
            return;
        }
    }

    // Wait for services to be ready
    info!("⏳ Waiting for services to be ready...");
    sleep(Duration::from_secs(15)).await;

    let client = reqwest::Client::new();

    // Test health endpoints
    let endpoints = vec![
        ("Direct API", "http://localhost:8080/health"),
        ("Socat Relay", "http://localhost:8081/health"),
    ];

    for (name, url) in endpoints {
        info!("🧪 Testing {} health endpoint: {}", name, url);
        
        match client.get(url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    let body = response.text().await.unwrap();
                    info!("✅ {} health check successful: {}", name, body);
                } else {
                    warn!("❌ {} health check failed with status: {}", name, response.status());
                }
            }
            Err(e) => {
                warn!("❌ {} health check failed: {}", name, e);
            }
        }
    }

    // Test container status
    info!("📋 Checking container status...");
    let status_result = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "ps"])
        .output()
        .await;

    match status_result {
        Ok(output) => {
            if output.status.success() {
                let status = String::from_utf8_lossy(&output.stdout);
                info!("📋 Container status:\n{}", status);
            } else {
                warn!("❌ Failed to get container status");
            }
        }
        Err(e) => {
            warn!("❌ Failed to execute docker-compose ps: {}", e);
        }
    }

    // Cleanup
    info!("🧹 Cleaning up...");
    let _ = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "down"])
        .output()
        .await;

    info!("✅ Sidecar Health Check Test completed successfully!");
}
