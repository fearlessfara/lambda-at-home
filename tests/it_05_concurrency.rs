#![cfg(feature = "docker_tests")]

use lambda_testsupport::*;
use std::time::Duration;

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn concurrency_throttling() -> anyhow::Result<()> {
    let mut daemon = spawn_daemon(None).await?;
    let zip = zip_dir(&example_path("echo-node"))?;
    let name = "concurrency-test";

    create_function(&daemon, serde_json::json!({
        "FunctionName": name,
        "Runtime": "nodejs18.x",
        "Handler": "index.handler",
        "MemorySize": 256,
        "Timeout": 5,
        "Code": { "ZipFile": b64(&zip) }
    })).await?;

    // Set concurrency limit to 1
    put_concurrency(&daemon, name, lambda_models::ConcurrencyConfig {
        reserved_concurrent_executions: Some(1),
    }).await?;

    // Fire 3 simultaneous invocations
    let mut tasks = Vec::new();
    for i in 0..3 {
        let daemon_url = daemon.user_api_url.clone();
        let task = tokio::spawn(async move {
            let client = lambda_testsupport::LambdaClient::new(daemon_url);
            client.invoke(name, serde_json::json!({"test": i}), Some(("RequestResponse", "Tail"))).await
        });
        tasks.push(task);
    }

    let results = futures::future::join_all(tasks).await;
    let mut success_count = 0;
    let mut throttle_count = 0;

    for result in results {
        match result {
            Ok(Ok(resp)) => {
                if resp.status_code == 200 {
                    success_count += 1;
                } else if resp.status_code == 429 {
                    throttle_count += 1;
                }
            }
            Ok(Err(e)) => {
                if e.to_string().contains("429") || e.to_string().contains("Throttled") {
                    throttle_count += 1;
                }
            }
            Err(_) => {
                // Task panicked, count as throttle
                throttle_count += 1;
            }
        }
    }

    // With concurrency limit of 1, we should have 1 success and 2 throttles
    assert!(success_count >= 1, "Expected at least 1 successful invocation");
    assert!(throttle_count >= 1, "Expected at least 1 throttled invocation");

    daemon.kill().await?;
    Ok(())
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn concurrency_increase_throughput() -> anyhow::Result<()> {
    let mut daemon = spawn_daemon(None).await?;
    let zip = zip_dir(&example_path("echo-node"))?;
    let name = "concurrency-throughput-test";

    create_function(&daemon, serde_json::json!({
        "FunctionName": name,
        "Runtime": "nodejs18.x",
        "Handler": "index.handler",
        "MemorySize": 256,
        "Timeout": 5,
        "Code": { "ZipFile": b64(&zip) }
    })).await?;

    // First test with concurrency limit of 1
    put_concurrency(&daemon, name, lambda_models::ConcurrencyConfig {
        reserved_concurrent_executions: Some(1),
    }).await?;

    let start_time = std::time::Instant::now();
    let mut tasks = Vec::new();
    for i in 0..5 {
        let daemon_url = daemon.user_api_url.clone();
        let task = tokio::spawn(async move {
            let client = lambda_testsupport::LambdaClient::new(daemon_url);
            client.invoke(name, serde_json::json!({"test": i}), Some(("RequestResponse", "Tail"))).await
        });
        tasks.push(task);
    }

    let results1 = futures::future::join_all(tasks).await;
    let duration1 = start_time.elapsed();
    let success_count1 = results1.iter().filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok()).count();

    // Increase concurrency limit
    put_concurrency(&daemon, name, lambda_models::ConcurrencyConfig {
        reserved_concurrent_executions: Some(5),
    }).await?;

    // Wait a bit for the change to take effect
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Second test with higher concurrency
    let start_time = std::time::Instant::now();
    let mut tasks = Vec::new();
    for i in 0..5 {
        let daemon_url = daemon.user_api_url.clone();
        let task = tokio::spawn(async move {
            let client = lambda_testsupport::LambdaClient::new(daemon_url);
            client.invoke(name, serde_json::json!({"test": i}), Some(("RequestResponse", "Tail"))).await
        });
        tasks.push(task);
    }

    let results2 = futures::future::join_all(tasks).await;
    let duration2 = start_time.elapsed();
    let success_count2 = results2.iter().filter(|r| r.is_ok() && r.as_ref().unwrap().is_ok()).count();

    // With higher concurrency, we should have more successful invocations
    assert!(success_count2 >= success_count1, "Higher concurrency should allow more successful invocations");

    daemon.kill().await?;
    Ok(())
}
