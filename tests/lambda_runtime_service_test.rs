use lambda_at_home::api::lambda_runtime_api::LambdaRuntimeService;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_runtime_service_comprehensive() {
    // Initialize logging
    let _ = env_logger::try_init();

    println!("🧪 Testing Lambda Runtime Service Comprehensive");

    // Create service with low concurrency limit for testing
    let service = Arc::new(LambdaRuntimeService::with_max_concurrency(2));
    let function_id = Uuid::new_v4();
    let container_id = "test-container-comprehensive".to_string();

    // Test 1: Container registration
    service.register_container(container_id.clone(), function_id).await;
    println!("✅ Container registered successfully");

    // Test 2: Basic invocation queuing and processing
    let payload1 = json!({"test": "basic_invocation"});
    let request_id1 = service.queue_invocation(function_id, payload1.clone()).await.unwrap();
    println!("✅ Basic invocation queued: {}", request_id1);

    // Test 3: RIC polling
    let invocation = service.get_next_invocation(&container_id).await.unwrap().unwrap();
    assert_eq!(invocation.request_id, request_id1);
    assert_eq!(invocation.function_id, function_id);
    assert_eq!(invocation.payload, payload1);
    println!("✅ Invocation retrieved via RIC polling");

    // Test 4: Concurrency limits (before submitting response for first invocation)
    let payload2 = json!({"test": "concurrency_test"});
    let request_id2 = service.queue_invocation(function_id, payload2).await.unwrap();
    println!("✅ Second invocation queued: {}", request_id2);

    // Retrieve the second invocation to make it active
    let invocation2 = service.get_next_invocation(&container_id).await.unwrap().unwrap();
    assert_eq!(invocation2.request_id, request_id2);
    println!("✅ Second invocation retrieved (now active)");

    // Third invocation should fail due to concurrency limit (2 active invocations)
    let payload3 = json!({"test": "concurrency_limit"});
    let result = service.queue_invocation(function_id, payload3).await;
    assert!(result.is_err());
    println!("✅ Third invocation correctly rejected due to concurrency limit");

    // Test 5: Response submission for first invocation
    let response = json!({"statusCode": 200, "body": "test_response"});
    service.submit_response(request_id1.clone(), response.clone()).await.unwrap();
    println!("✅ First invocation response submitted successfully");

    // Test 6: Response retrieval
    let retrieved_response = service.get_response(&request_id1).await.unwrap().unwrap();
    assert_eq!(retrieved_response.response, response);
    println!("✅ Response retrieved successfully");

    // Submit response for second invocation to free up concurrency
    let response2 = json!({"statusCode": 200, "body": "concurrency_test_response"});
    service.submit_response(request_id2.clone(), response2).await.unwrap();
    println!("✅ Second invocation response submitted");

    // Test 7: FIFO queueing
    let service_fifo = Arc::new(LambdaRuntimeService::new());
    let function_fifo_id = Uuid::new_v4();
    let container_fifo_id = "test-container-fifo".to_string();

    service_fifo.register_container(container_fifo_id.clone(), function_fifo_id).await;

    // Queue multiple invocations
    let mut request_ids = Vec::new();
    for i in 1..=3 {
        let payload = json!({"test": format!("fifo_invocation_{}", i)});
        let request_id = service_fifo.queue_invocation(function_fifo_id, payload).await.unwrap();
        request_ids.push(request_id.clone());
        println!("✅ FIFO invocation {} queued: {}", i, request_id);
    }

    // Retrieve in FIFO order
    for (i, expected_request_id) in request_ids.iter().enumerate() {
        let invocation = service_fifo.get_next_invocation(&container_fifo_id).await.unwrap().unwrap();
        assert_eq!(invocation.request_id, *expected_request_id);
        println!("✅ FIFO invocation {} retrieved in correct order", i + 1);
    }

    // Test 8: Error handling
    let service_error = Arc::new(LambdaRuntimeService::new());
    let function_error_id = Uuid::new_v4();
    let container_error_id = "test-container-error".to_string();

    service_error.register_container(container_error_id.clone(), function_error_id).await;

    let error_payload = json!({"test": "error_test"});
    let error_request_id = service_error.queue_invocation(function_error_id, error_payload).await.unwrap();
    println!("✅ Error test invocation queued: {}", error_request_id);

    let error_invocation = service_error.get_next_invocation(&container_error_id).await.unwrap().unwrap();
    assert_eq!(error_invocation.request_id, error_request_id);
    println!("✅ Error test invocation retrieved");

    // Submit error
    service_error.submit_error(
        error_request_id.clone(),
        "TestError".to_string(),
        "This is a test error".to_string(),
        Some(vec!["line 1".to_string(), "line 2".to_string()])
    ).await.unwrap();
    println!("✅ Error submitted successfully");

    // Verify error was received
    let received_error = service_error.get_error(&error_request_id).await.unwrap().unwrap();
    assert_eq!(received_error.error_type, "TestError");
    assert_eq!(received_error.error_message, "This is a test error");
    assert_eq!(received_error.stack_trace.unwrap().len(), 2);
    println!("✅ Error correctly received and validated");

    // Test 9: Multiple functions
    let service_multi = Arc::new(LambdaRuntimeService::with_max_concurrency(1));
    let function1_id = Uuid::new_v4();
    let function2_id = Uuid::new_v4();
    let container1_id = "test-container-multi-1".to_string();
    let container2_id = "test-container-multi-2".to_string();

    service_multi.register_container(container1_id.clone(), function1_id).await;
    service_multi.register_container(container2_id.clone(), function2_id).await;

    // Both functions should be able to queue invocations
    let payload1 = json!({"function": "1"});
    let request_id1 = service_multi.queue_invocation(function1_id, payload1).await.unwrap();
    println!("✅ Multi-function test: Function 1 invocation queued");

    let payload2 = json!({"function": "2"});
    let request_id2 = service_multi.queue_invocation(function2_id, payload2).await.unwrap();
    println!("✅ Multi-function test: Function 2 invocation queued");

    // Both should be retrievable
    let invocation1 = service_multi.get_next_invocation(&container1_id).await.unwrap().unwrap();
    assert_eq!(invocation1.request_id, request_id1);
    println!("✅ Multi-function test: Function 1 invocation retrieved");

    let invocation2 = service_multi.get_next_invocation(&container2_id).await.unwrap().unwrap();
    assert_eq!(invocation2.request_id, request_id2);
    println!("✅ Multi-function test: Function 2 invocation retrieved");

    // Submit responses
    let response1 = json!({"statusCode": 200, "body": "function1_response"});
    service_multi.submit_response(request_id1.clone(), response1).await.unwrap();
    println!("✅ Multi-function test: Function 1 response submitted");

    let response2 = json!({"statusCode": 200, "body": "function2_response"});
    service_multi.submit_response(request_id2.clone(), response2).await.unwrap();
    println!("✅ Multi-function test: Function 2 response submitted");

    println!("🎉 Lambda Runtime Service Comprehensive test passed!");
}

#[tokio::test]
async fn test_lambda_runtime_service_stress() {
    // Initialize logging
    let _ = env_logger::try_init();

    println!("🧪 Testing Lambda Runtime Service Stress");

    let service = Arc::new(LambdaRuntimeService::with_max_concurrency(10));
    let function_id = Uuid::new_v4();
    let container_id = "test-container-stress".to_string();

    service.register_container(container_id.clone(), function_id).await;

    // Queue many invocations rapidly
    let mut request_ids = Vec::new();
    for i in 0..20 {
        let payload = json!({"test": format!("stress_invocation_{}", i)});
        let request_id = service.queue_invocation(function_id, payload).await.unwrap();
        request_ids.push(request_id);
    }
    println!("✅ 20 invocations queued rapidly");

    // Process all invocations
    let mut responses = Vec::new();
    for (i, expected_request_id) in request_ids.iter().enumerate() {
        let invocation = service.get_next_invocation(&container_id).await.unwrap().unwrap();
        assert_eq!(invocation.request_id, *expected_request_id);
        
        let response = json!({"statusCode": 200, "body": format!("stress_response_{}", i)});
        service.submit_response(invocation.request_id.clone(), response.clone()).await.unwrap();
        responses.push(response);
    }
    println!("✅ All 20 invocations processed");

    // Verify all responses
    for (i, request_id) in request_ids.iter().enumerate() {
        let retrieved_response = service.get_response(request_id).await.unwrap().unwrap();
        assert_eq!(retrieved_response.response, responses[i]);
    }
    println!("✅ All 20 responses verified");

    println!("🎉 Lambda Runtime Service Stress test passed!");
}
