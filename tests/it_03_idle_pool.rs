#![cfg(feature = "docker_tests")]

use lambda_testsupport::*;
use std::time::Duration;

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn idle_stop_and_remove() -> anyhow::Result<()> {
    let cfg = ConfigOverride {
        idle_soft_ms: Some(2000),
        idle_hard_ms: Some(4000),
        ..Default::default()
    };
    let mut daemon = spawn_daemon(Some(cfg)).await?;
    let zip = zip_dir(&example_path("echo-node"))?;
    let name = "echo-idle-test";

    create_function(&daemon, serde_json::json!({
        "FunctionName": name,
        "Runtime": "nodejs18.x",
        "Handler": "index.handler",
        "MemorySize": 256,
        "Timeout": 5,
        "Code": { "ZipFile": b64(&zip) }
    })).await?;

    // First invocation - should be cold start
    let start_time = std::time::Instant::now();
    let resp1 = invoke(&daemon, name, serde_json::json!({"test":"first"}), Some(("RequestResponse","Tail"))).await?;
    let first_duration = start_time.elapsed();
    assert!(resp1.function_error.is_none());

    // Second invocation within soft idle window - should reuse warm container
    let start_time = std::time::Instant::now();
    let resp2 = invoke(&daemon, name, serde_json::json!({"test":"second"}), Some(("RequestResponse","Tail"))).await?;
    let second_duration = start_time.elapsed();
    assert!(resp2.function_error.is_none());

    // Second invocation should be faster (warm start)
    assert!(second_duration < first_duration, "Warm start should be faster than cold start");

    // Wait for soft idle timeout
    tokio::time::sleep(Duration::from_millis(2500)).await;

    // Third invocation after soft idle - should be cold start again
    let start_time = std::time::Instant::now();
    let resp3 = invoke(&daemon, name, serde_json::json!({"test":"third"}), Some(("RequestResponse","Tail"))).await?;
    let third_duration = start_time.elapsed();
    assert!(resp3.function_error.is_none());

    // Should be slower again (cold start)
    assert!(third_duration > second_duration, "Cold start after idle should be slower");

    daemon.kill().await?;
    Ok(())
}

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn warm_pool_reuse() -> anyhow::Result<()> {
    let mut daemon = spawn_daemon(None).await?;
    let zip = zip_dir(&example_path("echo-node"))?;
    let name = "echo-warm-test";

    create_function(&daemon, serde_json::json!({
        "FunctionName": name,
        "Runtime": "nodejs18.x",
        "Handler": "index.handler",
        "MemorySize": 256,
        "Timeout": 5,
        "Code": { "ZipFile": b64(&zip) }
    })).await?;

    // Multiple rapid invocations should reuse warm containers
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
    for result in results {
        let resp = result??;
        assert!(resp.function_error.is_none());
    }

    daemon.kill().await?;
    Ok(())
}
