#![cfg(feature = "docker_tests")]

use lambda_testsupport::*;

#[ignore]
#[tokio::test(flavor = "multi_thread")]
async fn smoke_test() -> anyhow::Result<()> {
    let mut daemon = spawn_daemon(None).await?;
    
    // Test health endpoint
    let client = reqwest::Client::new();
    let health_response = client
        .get(&format!("{}/healthz", daemon.user_api_url))
        .send()
        .await?;
    
    assert!(health_response.status().is_success());
    assert_eq!(health_response.text().await?, "OK");
    
    // Test metrics endpoint
    let metrics_response = client
        .get(&format!("{}/metrics", daemon.user_api_url))
        .send()
        .await?;
    
    assert!(metrics_response.status().is_success());
    let metrics_text = metrics_response.text().await?;
    assert!(metrics_text.contains("lambda_invocations_total"));
    
    // Parse metrics to verify structure
    let metrics = prom_parse(&metrics_text)?;
    assert!(metrics.counters.contains_key("lambda_invocations_total"));
    
    // Test runtime API health
    let runtime_health_response = client
        .get(&format!("{}/healthz", daemon.runtime_api_url))
        .send()
        .await?;
    
    assert!(runtime_health_response.status().is_success());
    assert_eq!(runtime_health_response.text().await?, "OK");
    
    daemon.kill().await?;
    Ok(())
}
