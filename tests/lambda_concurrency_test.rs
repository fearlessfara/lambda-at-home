use lambda_at_home::api::lambda_runtime_api::LambdaRuntimeService;
use serde_json::json;
use std::sync::Arc;

use uuid::Uuid;

#[tokio::test]
async fn test_lambda_concurrency_limits() {
    // Initialize logging
    let _ = env_logger::try_init();

    println!("🧪 Testing Lambda concurrency limits");

    // Create service with low concurrency limit for testing
    let service = Arc::new(LambdaRuntimeService::with_max_concurrency(2));
    let function_id = Uuid::new_v4();
    let container_id = "test-container-123".to_string();

    // Register container
    service.register_container(container_id.clone(), function_id).await;

    // Queue first invocation - should succeed
    let payload1 = json!({"test": "invocation1"});
    let request_id1 = service.queue_invocation(function_id, payload1).await.unwrap();
    println!("✅ First invocation queued: {}", request_id1);

    // Queue second invocation - should succeed
    let payload2 = json!({"test": "invocation2"});
    let request_id2 = service.queue_invocation(function_id, payload2).await.unwrap();
    println!("✅ Second invocation queued: {}", request_id2);

    // Simulate RIC processing first invocation
    let invocation1 = service.get_next_invocation(&container_id).await.unwrap().unwrap();
    assert_eq!(invocation1.request_id, request_id1);
    println!("✅ First invocation retrieved by RIC");

    // Simulate RIC processing second invocation
    let invocation2 = service.get_next_invocation(&container_id).await.unwrap().unwrap();
    assert_eq!(invocation2.request_id, request_id2);
    println!("✅ Second invocation retrieved by RIC");

    // Queue third invocation - should fail due to concurrency limit (2 active)
    let payload3 = json!({"test": "invocation3"});
    let result = service.queue_invocation(function_id, payload3.clone()).await;
    assert!(result.is_err());
    println!("✅ Third invocation correctly rejected due to concurrency limit");

    // Submit response for first invocation
    let response1 = json!({"statusCode": 200, "body": "response1"});
    service.submit_response(request_id1.clone(), response1).await.unwrap();
    println!("✅ First invocation response submitted");

    // Now third invocation should succeed
    let request_id3 = service.queue_invocation(function_id, payload3).await.unwrap();
    println!("✅ Third invocation queued after first completed: {}", request_id3);

    println!("🎉 Lambda concurrency limits test passed!");
}

#[tokio::test]
async fn test_lambda_fifo_queueing() {
    // Initialize logging
    let _ = env_logger::try_init();

    println!("🧪 Testing Lambda FIFO queueing");

    let service = Arc::new(LambdaRuntimeService::new());
    let function_id = Uuid::new_v4();
    let container_id = "test-container-456".to_string();

    // Register container
    service.register_container(container_id.clone(), function_id).await;

    // Queue multiple invocations
    let mut request_ids = Vec::new();
    for i in 1..=5 {
        let payload = json!({"test": format!("invocation{}", i)});
        let request_id = service.queue_invocation(function_id, payload).await.unwrap();
        request_ids.push(request_id);
        println!("✅ Queued invocation {}: {}", i, request_ids.last().unwrap());
    }

    // Retrieve invocations in order (FIFO)
    for (i, expected_request_id) in request_ids.iter().enumerate() {
        let invocation = service.get_next_invocation(&container_id).await.unwrap().unwrap();
        assert_eq!(invocation.request_id, *expected_request_id);
        println!("✅ Retrieved invocation {} in correct order: {}", i + 1, invocation.request_id);
    }

    // No more invocations should be available
    let no_invocation = service.get_next_invocation(&container_id).await.unwrap();
    assert!(no_invocation.is_none());
    println!("✅ No more invocations available (correct)");

    println!("🎉 Lambda FIFO queueing test passed!");
}

#[tokio::test]
async fn test_lambda_concurrent_functions() {
    // Initialize logging
    let _ = env_logger::try_init();

    println!("🧪 Testing Lambda concurrent functions");

    let service = Arc::new(LambdaRuntimeService::with_max_concurrency(1));
    
    // Create two different functions
    let function1_id = Uuid::new_v4();
    let function2_id = Uuid::new_v4();
    
    let container1_id = "test-container-1".to_string();
    let container2_id = "test-container-2".to_string();

    // Register containers for different functions
    service.register_container(container1_id.clone(), function1_id).await;
    service.register_container(container2_id.clone(), function2_id).await;

    // Queue invocations for both functions - both should succeed
    let payload1 = json!({"function": "1"});
    let request_id1 = service.queue_invocation(function1_id, payload1).await.unwrap();
    println!("✅ Function 1 invocation queued: {}", request_id1);

    let payload2 = json!({"function": "2"});
    let request_id2 = service.queue_invocation(function2_id, payload2).await.unwrap();
    println!("✅ Function 2 invocation queued: {}", request_id2);

    // Both functions should be able to process their invocations concurrently
    let invocation1 = service.get_next_invocation(&container1_id).await.unwrap().unwrap();
    assert_eq!(invocation1.request_id, request_id1);
    println!("✅ Function 1 invocation retrieved");

    let invocation2 = service.get_next_invocation(&container2_id).await.unwrap().unwrap();
    assert_eq!(invocation2.request_id, request_id2);
    println!("✅ Function 2 invocation retrieved");

    // Submit responses
    let response1 = json!({"statusCode": 200, "body": "function1_response"});
    service.submit_response(request_id1.clone(), response1).await.unwrap();
    println!("✅ Function 1 response submitted");

    let response2 = json!({"statusCode": 200, "body": "function2_response"});
    service.submit_response(request_id2.clone(), response2).await.unwrap();
    println!("✅ Function 2 response submitted");

    println!("🎉 Lambda concurrent functions test passed!");
}

#[tokio::test]
async fn test_lambda_error_handling() {
    // Initialize logging
    let _ = env_logger::try_init();

    println!("🧪 Testing Lambda error handling");

    let service = Arc::new(LambdaRuntimeService::new());
    let function_id = Uuid::new_v4();
    let container_id = "test-container-error".to_string();

    // Register container
    service.register_container(container_id.clone(), function_id).await;

    // Queue invocation
    let payload = json!({"test": "error_test"});
    let request_id = service.queue_invocation(function_id, payload).await.unwrap();
    println!("✅ Invocation queued: {}", request_id);

    // Retrieve invocation
    let invocation = service.get_next_invocation(&container_id).await.unwrap().unwrap();
    assert_eq!(invocation.request_id, request_id);
    println!("✅ Invocation retrieved");

    // Submit error instead of response
    service.submit_error(
        request_id.clone(),
        "TestError".to_string(),
        "This is a test error".to_string(),
        Some(vec!["line 1".to_string(), "line 2".to_string()])
    ).await.unwrap();
    println!("✅ Error submitted");

    // Check that error was received
    let received_error = service.get_error(&request_id).await.unwrap();
    assert!(received_error.is_some());
    
    let error = received_error.unwrap();
    assert_eq!(error.error_type, "TestError");
    assert_eq!(error.error_message, "This is a test error");
    assert_eq!(error.stack_trace.unwrap().len(), 2);
    println!("✅ Error correctly received and validated");

    // Check that no response was received
    let received_response = service.get_response(&request_id).await.unwrap();
    assert!(received_response.is_none());
    println!("✅ No response received (correct)");

    println!("🎉 Lambda error handling test passed!");
}

#[tokio::test]
async fn test_lambda_timeout_handling() {
    // Initialize logging
    let _ = env_logger::try_init();

    println!("🧪 Testing Lambda timeout handling");

    let service = Arc::new(LambdaRuntimeService::new());
    let function_id = Uuid::new_v4();
    let container_id = "test-container-timeout".to_string();

    // Register container
    service.register_container(container_id.clone(), function_id).await;

    // Queue invocation with short deadline
    let payload = json!({"test": "timeout_test"});
    let request_id = service.queue_invocation(function_id, payload).await.unwrap();
    println!("✅ Invocation queued: {}", request_id);

    // Retrieve invocation
    let invocation = service.get_next_invocation(&container_id).await.unwrap().unwrap();
    assert_eq!(invocation.request_id, request_id);
    
    // Check that deadline is set
    let now = chrono::Utc::now().timestamp_millis() as u64;
    assert!(invocation.deadline_ms > now);
    println!("✅ Invocation has valid deadline: {}", invocation.deadline_ms);

    // Submit response
    let response = json!({"statusCode": 200, "body": "timeout_test_response"});
    service.submit_response(request_id.clone(), response).await.unwrap();
    println!("✅ Response submitted");

    println!("🎉 Lambda timeout handling test passed!");
}
