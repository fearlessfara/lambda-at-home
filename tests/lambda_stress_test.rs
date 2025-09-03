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
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_stress_high_concurrency() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda High Concurrency Stress Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::with_max_concurrency(10));
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
    let function_name = format!("stress-test-{}", function_id);
    
    info!("📝 Creating stress test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Stress test function for high concurrency".to_string()),
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

    // Test high concurrency
    info!("🧪 Testing high concurrency Lambda execution");

    let num_concurrent = 10;
    let mut handles = Vec::new();
    let start_time = Instant::now();

    for i in 0..num_concurrent {
        let executor_clone = executor.clone();
        let function_id_clone = function_id;
        
        let handle = tokio::spawn(async move {
            let test_payload = json!({
                "message": format!("Stress test invocation {}", i),
                "number": i,
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "test_type": "stress_test",
                "invocation_id": i
            });

            let invoke_request = RICInvokeRequest {
                payload: test_payload.clone(),
                timeout: Some(30),
            };

            let invocation_start = Instant::now();
            
            match executor_clone.invoke_function(&function_id_clone, &invoke_request).await {
                Ok(response) => {
                    let duration = invocation_start.elapsed();
                    info!("✅ Stress invocation {} completed in {:?}", i, duration);
                    
                    assert_eq!(response.status_code, 200);
                    assert!(response.payload.is_some());
                    
                    let body = response.payload.unwrap();
                    if let Some(body_obj) = body.as_object() {
                        let computation_result = body_obj.get("computation_result").unwrap().as_f64().unwrap();
                        assert_eq!(computation_result, (i * 42) as f64);
                    }
                    
                    Ok(duration)
                }
                Err(e) => {
                    warn!("❌ Stress invocation {} failed: {}", i, e);
                    Err(e)
                }
            }
        });
        
        handles.push(handle);
    }

    // Wait for all invocations to complete
    let mut results = Vec::new();
    let mut total_duration = std::time::Duration::new(0, 0);
    let mut success_count = 0;

    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(duration)) => {
                info!("✅ Stress invocation {} completed successfully in {:?}", i, duration);
                results.push(Ok(duration));
                total_duration += duration;
                success_count += 1;
            }
            Ok(Err(e)) => {
                warn!("❌ Stress invocation {} failed: {}", i, e);
                results.push(Err(e));
            }
            Err(e) => {
                warn!("❌ Stress invocation {} task failed: {}", i, e);
                results.push(Err(anyhow::anyhow!("Task failed: {}", e)));
            }
        }
    }

    let total_time = start_time.elapsed();
    let success_rate = (success_count as f64 / num_concurrent as f64) * 100.0;
    let avg_duration = if success_count > 0 {
        total_duration / success_count
    } else {
        std::time::Duration::new(0, 0)
    };

    info!("📊 Stress Test Results:");
    info!("   Total invocations: {}", num_concurrent);
    info!("   Successful invocations: {}", success_count);
    info!("   Success rate: {:.1}%", success_rate);
    info!("   Total time: {:?}", total_time);
    info!("   Average duration: {:?}", avg_duration);
    info!("   Throughput: {:.2} invocations/second", num_concurrent as f64 / total_time.as_secs_f64());

    // Verify at least 80% success rate
    assert!(success_rate >= 80.0, "Success rate {}% is below 80%", success_rate);

    info!("🎉 Lambda High Concurrency Stress Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_stress_rapid_fire() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Rapid Fire Stress Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::with_max_concurrency(5));
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
    let function_name = format!("rapid-fire-test-{}", function_id);
    
    info!("📝 Creating rapid fire test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Rapid fire test function".to_string()),
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

    // Test rapid fire invocations
    info!("🧪 Testing rapid fire Lambda execution");

    let num_invocations = 20;
    let mut results = Vec::new();
    let start_time = Instant::now();

    // Execute invocations sequentially but rapidly
    for i in 0..num_invocations {
        let test_payload = json!({
            "message": format!("Rapid fire invocation {}", i),
            "number": i,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "test_type": "rapid_fire",
            "invocation_id": i
        });

        let invoke_request = RICInvokeRequest {
            payload: test_payload.clone(),
            timeout: Some(30),
        };

        let invocation_start = Instant::now();
        
        match executor.invoke_function(&function_id, &invoke_request).await {
            Ok(response) => {
                let duration = invocation_start.elapsed();
                info!("✅ Rapid fire invocation {} completed in {:?}", i, duration);
                
                assert_eq!(response.status_code, 200);
                assert!(response.payload.is_some());
                
                let body = response.payload.unwrap();
                if let Some(body_obj) = body.as_object() {
                    let computation_result = body_obj.get("computation_result").unwrap().as_f64().unwrap();
                    assert_eq!(computation_result, (i * 42) as f64);
                }
                
                results.push(Ok(duration));
            }
            Err(e) => {
                warn!("❌ Rapid fire invocation {} failed: {}", i, e);
                results.push(Err(e));
            }
        }
    }

    let total_time = start_time.elapsed();
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let success_rate = (success_count as f64 / num_invocations as f64) * 100.0;

    info!("📊 Rapid Fire Test Results:");
    info!("   Total invocations: {}", num_invocations);
    info!("   Successful invocations: {}", success_count);
    info!("   Success rate: {:.1}%", success_rate);
    info!("   Total time: {:?}", total_time);
    info!("   Throughput: {:.2} invocations/second", num_invocations as f64 / total_time.as_secs_f64());

    // Verify at least 90% success rate for rapid fire
    assert!(success_rate >= 90.0, "Success rate {}% is below 90%", success_rate);

    info!("🎉 Lambda Rapid Fire Stress Test completed successfully!");
}

#[tokio::test]
async fn test_lambda_stress_mixed_workload() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Mixed Workload Stress Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::with_max_concurrency(8));
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

    // Create multiple test functions with different configurations
    let function1_id = Uuid::new_v4();
    let function2_id = Uuid::new_v4();
    let function3_id = Uuid::new_v4();
    
    info!("📝 Creating mixed workload test functions");

    // Create function 1 (lightweight)
    let function1_metadata = Function {
        id: function1_id,
        name: format!("mixed-workload-1-{}", function1_id),
        description: Some("Lightweight function for mixed workload".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function1_id)),
        memory_size: Some(128),
        cpu_limit: Some(0.5),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Create function 2 (medium)
    let function2_metadata = Function {
        id: function2_id,
        name: format!("mixed-workload-2-{}", function2_id),
        description: Some("Medium function for mixed workload".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function2_id)),
        memory_size: Some(256),
        cpu_limit: Some(1.0),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Create function 3 (heavy)
    let function3_metadata = Function {
        id: function3_id,
        name: format!("mixed-workload-3-{}", function3_id),
        description: Some("Heavy function for mixed workload".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function3_id)),
        memory_size: Some(512),
        cpu_limit: Some(2.0),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Store function metadata
    storage.save_function(&function1_metadata).await.expect("Failed to store function 1");
    storage.save_function(&function2_metadata).await.expect("Failed to store function 2");
    storage.save_function(&function3_metadata).await.expect("Failed to store function 3");

    // Test mixed workload
    info!("🧪 Testing mixed workload Lambda execution");

    let mut handles = Vec::new();
    let start_time = Instant::now();

    // Create a mix of concurrent invocations across different functions
    let workloads = vec![
        (function1_id, 5, "lightweight"),
        (function2_id, 3, "medium"),
        (function3_id, 2, "heavy"),
    ];

    for (function_id, count, workload_type) in workloads {
        for i in 0..count {
            let executor_clone = executor.clone();
            let function_id_clone = function_id;
            
            let handle = tokio::spawn(async move {
                let test_payload = json!({
                    "message": format!("Mixed workload {} invocation {}", workload_type, i),
                    "number": i,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "test_type": "mixed_workload",
                    "workload_type": workload_type,
                    "invocation_id": i
                });

                let invoke_request = RICInvokeRequest {
                    payload: test_payload.clone(),
                    timeout: Some(30),
                };

                let invocation_start = Instant::now();
                
                match executor_clone.invoke_function(&function_id_clone, &invoke_request).await {
                    Ok(response) => {
                        let duration = invocation_start.elapsed();
                        info!("✅ Mixed workload {} invocation {} completed in {:?}", workload_type, i, duration);
                        
                        assert_eq!(response.status_code, 200);
                        assert!(response.payload.is_some());
                        
                        let body = response.payload.unwrap();
                        if let Some(body_obj) = body.as_object() {
                            let computation_result = body_obj.get("computation_result").unwrap().as_f64().unwrap();
                            assert_eq!(computation_result, (i * 42) as f64);
                        }
                        
                        Ok((workload_type, duration))
                    }
                    Err(e) => {
                        warn!("❌ Mixed workload {} invocation {} failed: {}", workload_type, i, e);
                        Err(e)
                    }
                }
            });
            
            handles.push(handle);
        }
    }

    // Wait for all invocations to complete
    let mut results = Vec::new();
    let mut workload_stats = std::collections::HashMap::new();

    for handle in handles.into_iter() {
        match handle.await {
            Ok(Ok((workload_type, duration))) => {
                info!("✅ Mixed workload {} completed successfully in {:?}", workload_type, duration);
                results.push(Ok(duration));
                
                let stats = workload_stats.entry(workload_type).or_insert((0, std::time::Duration::new(0, 0)));
                stats.0 += 1;
                stats.1 += duration;
            }
            Ok(Err(e)) => {
                warn!("❌ Mixed workload invocation failed: {}", e);
                results.push(Err(e));
            }
            Err(e) => {
                warn!("❌ Mixed workload task failed: {}", e);
                results.push(Err(anyhow::anyhow!("Task failed: {}", e)));
            }
        }
    }

    let total_time = start_time.elapsed();
    let success_count = results.iter().filter(|r| r.is_ok()).count();
    let success_rate = (success_count as f64 / results.len() as f64) * 100.0;

    info!("📊 Mixed Workload Test Results:");
    info!("   Total invocations: {}", results.len());
    info!("   Successful invocations: {}", success_count);
    info!("   Success rate: {:.1}%", success_rate);
    info!("   Total time: {:?}", total_time);
    info!("   Throughput: {:.2} invocations/second", results.len() as f64 / total_time.as_secs_f64());

    // Print workload-specific stats
    for (workload_type, (count, total_duration)) in workload_stats {
        let avg_duration = if count > 0 {
            total_duration / count
        } else {
            std::time::Duration::new(0, 0)
        };
        info!("   {} workload: {} invocations, avg duration: {:?}", workload_type, count, avg_duration);
    }

    // Verify at least 85% success rate for mixed workload
    assert!(success_rate >= 85.0, "Success rate {}% is below 85%", success_rate);

    info!("🎉 Lambda Mixed Workload Stress Test completed successfully!");
}
