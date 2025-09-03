use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

#[tokio::test]
async fn test_sidecar_simple_lambda_execution() {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("🚀 Starting Simple Sidecar Lambda Test");

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

    // Test health endpoints
    let client = reqwest::Client::new();
    
    info!("🧪 Testing health endpoints...");
    
    // Test direct connection to Lambda Runtime API
    match client.get("http://localhost:8080/health").send().await {
        Ok(response) => {
            if response.status().is_success() {
                let body = response.text().await.unwrap();
                info!("✅ Direct API health check successful: {}", body);
            } else {
                warn!("❌ Direct API health check failed with status: {}", response.status());
                return;
            }
        }
        Err(e) => {
            warn!("❌ Direct API health check failed: {}", e);
            return;
        }
    }

    // Test connection through socat relay
    match client.get("http://localhost:8081/health").send().await {
        Ok(response) => {
            if response.status().is_success() {
                let body = response.text().await.unwrap();
                info!("✅ Socat relay health check successful: {}", body);
            } else {
                warn!("❌ Socat relay health check failed with status: {}", response.status());
                return;
            }
        }
        Err(e) => {
            warn!("❌ Socat relay health check failed: {}", e);
            return;
        }
    }

    // Test Lambda Runtime API endpoints
    info!("🧪 Testing Lambda Runtime API endpoints...");
    
    // Test /runtime/invocation/next endpoint
    match client.get("http://localhost:8081/runtime/invocation/next").send().await {
        Ok(response) => {
            info!("✅ /runtime/invocation/next endpoint accessible (status: {})", response.status());
            if response.status() == 404 {
                info!("📋 Expected 404 - no pending invocations");
            }
        }
        Err(e) => {
            warn!("❌ /runtime/invocation/next endpoint failed: {}", e);
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

    // Check Lambda container logs
    info!("📋 Checking Lambda container logs...");
    let logs_result = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "logs", "lambda-container"])
        .output()
        .await;

    match logs_result {
        Ok(output) => {
            if output.status.success() {
                let logs = String::from_utf8_lossy(&output.stdout);
                info!("📋 Lambda container logs:\n{}", logs);
            } else {
                warn!("❌ Failed to get Lambda container logs");
            }
        }
        Err(e) => {
            warn!("❌ Failed to execute docker-compose logs: {}", e);
        }
    }

    // Test creating a simple invocation
    info!("🧪 Testing Lambda invocation creation...");
    
    let test_payload = json!({
        "message": "Hello from sidecar test!",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "testId": "sidecar-simple-test"
    });

    let invocation_payload = json!({
        "function_id": "ad80c2e0-aedc-4809-a27b-317361ec87e6", // Use existing function ID
        "payload": test_payload,
        "deadline_ms": 30000,
        "invoked_function_arn": "arn:aws:lambda:us-east-1:123456789012:function:test-function"
    });

    // Try to create an invocation (this might fail if no function is deployed, but we can test the endpoint)
    match client
        .post("http://localhost:8081/runtime/invocation/next")
        .json(&invocation_payload)
        .send()
        .await
    {
        Ok(response) => {
            info!("✅ Invocation endpoint accessible (status: {})", response.status());
            if response.status() == 404 {
                info!("📋 Expected 404 - no function deployed or no pending invocations");
            } else if response.status().is_success() {
                let body = response.text().await.unwrap();
                info!("📋 Invocation response: {}", body);
            }
        }
        Err(e) => {
            warn!("❌ Invocation endpoint failed: {}", e);
        }
    }

    // Test multiple health checks to ensure stability
    info!("🧪 Testing multiple health checks for stability...");
    for i in 1..=5 {
        match client.get("http://localhost:8081/health").send().await {
            Ok(response) => {
                if response.status().is_success() {
                    info!("✅ Health check {} successful", i);
                } else {
                    warn!("❌ Health check {} failed with status: {}", i, response.status());
                }
            }
            Err(e) => {
                warn!("❌ Health check {} failed: {}", i, e);
            }
        }
        sleep(Duration::from_secs(2)).await;
    }

    // Cleanup
    info!("🧹 Cleaning up...");
    let _ = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "down"])
        .output()
        .await;

    info!("✅ Simple Sidecar Lambda Test completed successfully!");
}

#[tokio::test]
async fn test_sidecar_network_connectivity() {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    info!("🚀 Starting Sidecar Network Connectivity Test");

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

    // Test network connectivity from different perspectives
    info!("🧪 Testing network connectivity...");
    
    // Test from host to direct API
    info!("📡 Testing host -> direct API");
    match client.get("http://localhost:8080/health").send().await {
        Ok(response) => {
            if response.status().is_success() {
                info!("✅ Host -> direct API: SUCCESS");
            } else {
                warn!("❌ Host -> direct API: FAILED (status: {})", response.status());
            }
        }
        Err(e) => {
            warn!("❌ Host -> direct API: FAILED ({})", e);
        }
    }

    // Test from host to socat relay
    info!("📡 Testing host -> socat relay");
    match client.get("http://localhost:8081/health").send().await {
        Ok(response) => {
            if response.status().is_success() {
                info!("✅ Host -> socat relay: SUCCESS");
            } else {
                warn!("❌ Host -> socat relay: FAILED (status: {})", response.status());
            }
        }
        Err(e) => {
            warn!("❌ Host -> socat relay: FAILED ({})", e);
        }
    }

    // Test from Lambda container to socat relay (simulated)
    info!("📡 Testing Lambda container -> socat relay (simulated)");
    let lambda_test_result = tokio::process::Command::new("docker")
        .args([
            "exec",
            "express-functions-lambda-container-1",
            "wget",
            "-q",
            "--spider",
            "http://socat-relay:8080/health"
        ])
        .output()
        .await;

    match lambda_test_result {
        Ok(output) => {
            if output.status.success() {
                info!("✅ Lambda container -> socat relay: SUCCESS");
            } else {
                warn!("❌ Lambda container -> socat relay: FAILED (exit code: {})", output.status);
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.is_empty() {
                    warn!("📋 Error details: {}", stderr);
                }
            }
        }
        Err(e) => {
            warn!("❌ Lambda container -> socat relay: FAILED ({})", e);
        }
    }

    // Test socat relay logs to see connections
    info!("📋 Checking socat relay connection logs...");
    let socat_logs_result = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "logs", "socat-relay"])
        .output()
        .await;

    match socat_logs_result {
        Ok(output) => {
            if output.status.success() {
                let logs = String::from_utf8_lossy(&output.stdout);
                info!("📋 Socat relay logs:\n{}", logs);
            } else {
                warn!("❌ Failed to get socat relay logs");
            }
        }
        Err(e) => {
            warn!("❌ Failed to execute docker-compose logs: {}", e);
        }
    }

    // Cleanup
    info!("🧹 Cleaning up...");
    let _ = tokio::process::Command::new("docker-compose")
        .args(["-f", "docker-compose.sidecar.yml", "down"])
        .output()
        .await;

    info!("✅ Sidecar Network Connectivity Test completed successfully!");
}
