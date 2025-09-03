use lambda_at_home::api::api::lambda_runtime_api::LambdaRuntimeService;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_runtime_api() {
    // Initialize logging
    let _ = env_logger::try_init();

    let service = Arc::new(LambdaRuntimeService::new());
    let function_id = Uuid::new_v4();
    let container_id = "test-container-123".to_string();

    // Register container
    service.register_container(container_id.clone(), function_id).await;

    // Queue an invocation
    let payload = json!({
        "message": "Hello from Lambda Runtime API test!"
    });
    
    let request_id = service.queue_invocation(function_id, payload).await.unwrap();
    println!("Queued invocation: {}", request_id);

    // Simulate RIC polling for next invocation
    let invocation = service.get_next_invocation(&container_id).await.unwrap();
    assert!(invocation.is_some());
    
    let invocation = invocation.unwrap();
    assert_eq!(invocation.request_id, request_id);
    assert_eq!(invocation.function_id, function_id);
    assert_eq!(invocation.payload["message"], "Hello from Lambda Runtime API test!");

    // Simulate RIC submitting response
    let response = json!({
        "statusCode": 200,
        "body": "Hello from function!"
    });
    
    service.submit_response(request_id.clone(), response.clone()).await.unwrap();

    // Check that response was received
    let received_response = service.get_response(&request_id).await.unwrap();
    assert!(received_response.is_some());
    assert_eq!(received_response.unwrap().response, response);

    println!("✅ Lambda Runtime API test passed!");
}

#[tokio::test]
async fn test_lambda_runtime_api_error() {
    // Initialize logging
    let _ = env_logger::try_init();

    let service = Arc::new(LambdaRuntimeService::new());
    let function_id = Uuid::new_v4();
    let container_id = "test-container-456".to_string();

    // Register container
    service.register_container(container_id.clone(), function_id).await;

    // Queue an invocation
    let payload = json!({"test": "error"});
    let request_id = service.queue_invocation(function_id, payload).await.unwrap();

    // Get the invocation
    let invocation = service.get_next_invocation(&container_id).await.unwrap().unwrap();

    // Simulate RIC submitting error
    service.submit_error(
        request_id.clone(),
        "TestError".to_string(),
        "This is a test error".to_string(),
        Some(vec!["line 1".to_string(), "line 2".to_string()])
    ).await.unwrap();

    // Check that error was received
    let received_error = service.get_error(&request_id).await.unwrap();
    assert!(received_error.is_some());
    
    let error = received_error.unwrap();
    assert_eq!(error.error_type, "TestError");
    assert_eq!(error.error_message, "This is a test error");
    assert_eq!(error.stack_trace.unwrap().len(), 2);

    println!("✅ Lambda Runtime API error test passed!");
}
