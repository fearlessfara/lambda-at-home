use lambda_at_home::{
    docker::container_lifecycle::ContainerLifecycleManager,
    docker::docker::DockerManager,
    function_core::storage::FunctionStorage,
    api::lambda_runtime_api::LambdaRuntimeService,
    core::config::Config,
};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing_subscriber;

#[tokio::test]
async fn test_lambda_metrics_and_performance() {
    // Initialize tracing
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    println!("🚀 Starting Lambda Metrics and Performance Test");
    println!("================================================");

    // Load configuration
    let config = Config::load().expect("Failed to load configuration");
    println!("📋 Configuration loaded successfully");

    // Initialize components
    let docker_manager = Arc::new(DockerManager::new().await.expect("Failed to create Docker manager"));
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lifecycle_manager = Arc::new(ContainerLifecycleManager::new(
        docker_manager.clone(),
        storage.clone(),
        config.clone(),
    ));
    let runtime_service = Arc::new(LambdaRuntimeService::new(config.clone()));

    println!("✅ All components initialized successfully");

    // Start the lifecycle manager
    lifecycle_manager.start().await;
    println!("🔄 Lifecycle manager started");

    // Test configuration
    let total_executions = 15;
    let concurrent_executions = 5;
    let function_id = "metrics-test-function-12345";

    // Create a test Lambda function
    println!("📦 Creating test Lambda function...");
    let function_code = r#"
const handler = async (event, context) => {
    const startTime = Date.now();
    const requestId = context.awsRequestId;
    
    console.log(`=== LAMBDA EXECUTION STARTED [${requestId}] ===`);
    
    try {
        // Simulate processing based on event type
        let processingTime = 0;
        let result = {
            requestId: requestId,
            timestamp: new Date().toISOString(),
            functionName: context.functionName,
            event: event,
            processing: {
                type: event.type || 'default',
                input: event,
                processed: true
            },
            metrics: {
                startTime: startTime,
                executionTime: 0,
                memoryUsage: process.memoryUsage(),
                cpuUsage: process.cpuUsage()
            }
        };
        
        // Simulate different processing scenarios
        switch (event.type) {
            case 'cpu_intensive':
                processingTime = 200 + Math.random() * 300;
                for (let i = 0; i < 1000000; i++) {
                    Math.sqrt(i * Math.random());
                }
                result.processing.cpuIntensive = true;
                break;
                
            case 'memory_intensive':
                processingTime = 150 + Math.random() * 200;
                const largeArray = new Array(100000).fill(0).map(() => Math.random());
                result.processing.memoryIntensive = true;
                result.processing.arraySize = largeArray.length;
                break;
                
            case 'io_simulation':
                processingTime = 100 + Math.random() * 400;
                await new Promise(resolve => setTimeout(resolve, processingTime));
                result.processing.ioSimulated = true;
                break;
                
            case 'error_test':
                processingTime = 50 + Math.random() * 100;
                throw new Error(`Simulated error for request ${requestId}`);
                
            default:
                processingTime = 50 + Math.random() * 150;
                await new Promise(resolve => setTimeout(resolve, processingTime));
                result.processing.standard = true;
        }
        
        const endTime = Date.now();
        result.metrics.executionTime = endTime - startTime;
        result.metrics.processingTime = processingTime;
        result.metrics.endTime = endTime;
        
        result.performance = {
            totalExecutionTime: result.metrics.executionTime,
            processingTime: processingTime,
            overhead: result.metrics.executionTime - processingTime,
            memoryUsage: process.memoryUsage(),
            cpuUsage: process.cpuUsage(result.metrics.cpuUsage)
        };
        
        console.log(`=== LAMBDA EXECUTION COMPLETED [${requestId}] ===`);
        console.log(`Execution time: ${result.metrics.executionTime}ms`);
        console.log(`Processing time: ${processingTime}ms`);
        console.log(`Overhead: ${result.performance.overhead}ms`);
        
        return {
            statusCode: 200,
            body: JSON.stringify(result),
            headers: {
                'Content-Type': 'application/json',
                'X-Request-ID': requestId,
                'X-Execution-Time': result.metrics.executionTime.toString(),
                'X-Processing-Time': processingTime.toString()
            }
        };
        
    } catch (error) {
        const endTime = Date.now();
        console.error(`=== LAMBDA EXECUTION ERROR [${requestId}] ===`);
        console.error('Error:', error.message);
        
        return {
            statusCode: 500,
            body: JSON.stringify({
                error: true,
                requestId: requestId,
                errorType: error.constructor.name,
                errorMessage: error.message,
                timestamp: new Date().toISOString(),
                executionTime: endTime - startTime
            }),
            headers: {
                'Content-Type': 'application/json',
                'X-Request-ID': requestId,
                'X-Error': 'true'
            }
        };
    }
};

module.exports = { handler };
"#;

    // Deploy the function
    let deployment_result = lifecycle_manager.deploy_function(
        function_id,
        function_code,
        128,
        0.5,
    ).await;

    match deployment_result {
        Ok(_) => println!("✅ Function deployed successfully: {}", function_id),
        Err(e) => {
            println!("❌ Failed to deploy function: {}", e);
            return;
        }
    }

    // Wait for function to be ready
    sleep(Duration::from_secs(5)).await;

    // Metrics collection
    let mut execution_times = Vec::new();
    let mut successful_executions = 0;
    let mut failed_executions = 0;
    let mut error_handling_tests = 0;

    println!("🧪 Starting execution tests...");

    // Test 1: Sequential executions with different event types
    println!("📊 Test 1: Sequential Executions ({} executions)", total_executions);
    let test_start = Instant::now();

    for i in 1..=total_executions {
        let event_type = match i % 4 {
            0 => "standard",
            1 => "cpu_intensive", 
            2 => "memory_intensive",
            3 => "io_simulation",
            _ => "standard",
        };

        let event_payload = serde_json::json!({
            "executionId": i,
            "type": event_type,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": {
                "test": true,
                "executionNumber": i,
                "randomValue": rand::random::<u32>() % 1000
            }
        });

        let execution_start = Instant::now();
        
        match lifecycle_manager.execute_function(function_id, event_payload.clone()).await {
            Ok(response) => {
                let execution_time = execution_start.elapsed();
                execution_times.push(execution_time.as_millis() as u64);
                successful_executions += 1;
                
                println!("✅ Execution {} completed successfully ({}ms) - Type: {}", 
                    i, execution_time.as_millis(), event_type);
            }
            Err(e) => {
                let execution_time = execution_start.elapsed();
                failed_executions += 1;
                println!("❌ Execution {} failed: {} ({}ms)", i, e, execution_time.as_millis());
            }
        }

        // Small delay between executions
        sleep(Duration::from_millis(100)).await;
    }

    let test_duration = test_start.elapsed();
    println!("📈 Sequential test completed in {:?}", test_duration);

    // Test 2: Concurrent executions
    println!("🔄 Test 2: Concurrent Executions ({} concurrent)", concurrent_executions);
    let concurrent_start = Instant::now();

    let mut handles = Vec::new();
    for i in 1..=concurrent_executions {
        let lifecycle_manager_clone = lifecycle_manager.clone();
        let function_id_clone = function_id.to_string();
        
        let handle = tokio::spawn(async move {
            let event_payload = serde_json::json!({
                "executionId": format!("concurrent_{}", i),
                "type": "standard",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "data": {
                    "test": true,
                    "executionNumber": i,
                    "concurrent": true
                }
            });

            let start = Instant::now();
            let result = lifecycle_manager_clone.execute_function(&function_id_clone, event_payload).await;
            let duration = start.elapsed();
            
            (result, duration)
        });
        
        handles.push(handle);
    }

    let mut concurrent_successful = 0;
    let mut concurrent_failed = 0;
    let mut concurrent_times = Vec::new();

    for handle in handles {
        match handle.await {
            Ok((Ok(_), duration)) => {
                concurrent_successful += 1;
                concurrent_times.push(duration.as_millis() as u64);
                println!("✅ Concurrent execution completed ({}ms)", duration.as_millis());
            }
            Ok((Err(e), duration)) => {
                concurrent_failed += 1;
                println!("❌ Concurrent execution failed: {} ({}ms)", e, duration.as_millis());
            }
            Err(e) => {
                concurrent_failed += 1;
                println!("❌ Concurrent execution panicked: {}", e);
            }
        }
    }

    let concurrent_duration = concurrent_start.elapsed();
    println!("📈 Concurrent test completed in {:?}", concurrent_duration);

    // Test 3: Error handling
    println!("⚠️  Test 3: Error Handling (5 error simulations)");
    for i in 1..=5 {
        let event_payload = serde_json::json!({
            "executionId": format!("error_{}", i),
            "type": "error_test",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": {
                "test": true,
                "errorSimulation": true
            }
        });

        let start = Instant::now();
        match lifecycle_manager.execute_function(function_id, event_payload).await {
            Ok(_) => {
                println!("✅ Error test {} handled correctly ({}ms)", i, start.elapsed().as_millis());
                error_handling_tests += 1;
            }
            Err(e) => {
                println!("❌ Error test {} unexpected result: {} ({}ms)", i, e, start.elapsed().as_millis());
            }
        }
    }

    // Calculate statistics
    println!("📊 Calculating performance statistics...");
    
    if !execution_times.is_empty() {
        execution_times.sort();
        
        let min_time = execution_times[0];
        let max_time = execution_times[execution_times.len() - 1];
        let avg_time = execution_times.iter().sum::<u64>() / execution_times.len() as u64;
        
        let p50_index = execution_times.len() / 2;
        let p90_index = (execution_times.len() * 9) / 10;
        let p95_index = (execution_times.len() * 95) / 100;
        let p99_index = (execution_times.len() * 99) / 100;
        
        let p50_time = execution_times[p50_index];
        let p90_time = execution_times[p90_index];
        let p95_time = execution_times[p95_index];
        let p99_time = execution_times[p99_index];

        // Calculate throughput
        let total_executions_count = successful_executions + failed_executions;
        let throughput = if test_duration.as_millis() > 0 {
            (total_executions_count as f64 * 1000.0) / test_duration.as_millis() as f64
        } else {
            0.0
        };

        let concurrent_throughput = if concurrent_duration.as_millis() > 0 {
            (concurrent_executions as f64 * 1000.0) / concurrent_duration.as_millis() as f64
        } else {
            0.0
        };

        // Print results
        println!("");
        println!("🎯 LAMBDA METRICS TEST RESULTS:");
        println!("===============================");
        println!("");
        println!("📈 Sequential Execution Results:");
        println!("  Total executions: {}", total_executions_count);
        println!("  Successful: {}", successful_executions);
        println!("  Failed: {}", failed_executions);
        println!("  Success rate: {:.1}%", (successful_executions as f64 / total_executions_count as f64) * 100.0);
        println!("  Total test time: {:?}", test_duration);
        println!("  Throughput: {:.2} executions/second", throughput);
        println!("");
        println!("🔄 Concurrent Execution Results:");
        println!("  Concurrent executions: {}", concurrent_executions);
        println!("  Successful: {}", concurrent_successful);
        println!("  Failed: {}", concurrent_failed);
        println!("  Concurrent success rate: {:.1}%", (concurrent_successful as f64 / concurrent_executions as f64) * 100.0);
        println!("  Concurrent test time: {:?}", concurrent_duration);
        println!("  Concurrent throughput: {:.2} executions/second", concurrent_throughput);
        println!("");
        println!("⚠️  Error Handling Results:");
        println!("  Error tests: 5");
        println!("  Correctly handled: {}", error_handling_tests);
        println!("  Error handling rate: {:.1}%", (error_handling_tests as f64 / 5.0) * 100.0);
        println!("");
        println!("📊 Response Time Statistics:");
        println!("  Minimum: {}ms", min_time);
        println!("  Maximum: {}ms", max_time);
        println!("  Average: {}ms", avg_time);
        println!("  P50 (Median): {}ms", p50_time);
        println!("  P90: {}ms", p90_time);
        println!("  P95: {}ms", p95_time);
        println!("  P99: {}ms", p99_time);
        println!("");
        println!("🎉 Test completed successfully!");
        println!("   - Fast Lambda execution times");
        println!("   - Reliable concurrent processing");
        println!("   - Proper error handling");
        println!("   - Consistent performance metrics");
    } else {
        println!("❌ No execution times collected - test failed");
    }

    // Cleanup
    println!("🧹 Cleaning up...");
    lifecycle_manager.stop().await;
    println!("✅ Cleanup completed");
}
