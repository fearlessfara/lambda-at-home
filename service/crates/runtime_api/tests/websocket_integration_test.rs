use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use lambda_control::{pending::Pending, queues::Queues};
use lambda_runtime_api::{build_router, RtState};
use serde_json::json;
use tower::ServiceExt;
// WebSocket integration tests

#[tokio::test]
async fn test_websocket_endpoint_exists() {
    // Create test state
    let state = RtState {
        control: None,
        queues: Queues::new(),
        pending: Pending::new(),
    };

    let app = Router::new().merge(build_router(state));

    // Test that the WebSocket endpoint exists and responds
    let request = Request::builder()
        .uri("/2018-06-01/runtime/websocket?fn=test-function")
        .header("upgrade", "websocket")
        .header("connection", "upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // The endpoint should exist and respond (either with upgrade or error)
    assert!(response.status() == StatusCode::SWITCHING_PROTOCOLS || 
            response.status() == StatusCode::BAD_REQUEST ||
            response.status() == StatusCode::UPGRADE_REQUIRED);
}

#[tokio::test]
async fn test_websocket_message_serialization() {
    use lambda_runtime_api::websocket::WebSocketMessage;
    use serde_json;

    // Test all message types can be serialized and deserialized
    let messages = vec![
        WebSocketMessage::Register {
            function_name: "test-function".to_string(),
            runtime: Some("nodejs18.x".to_string()),
            version: Some("1".to_string()),
            env_hash: Some("abc123".to_string()),
            instance_id: Some("instance-123".to_string()),
        },
        WebSocketMessage::Invocation {
            request_id: "req-123".to_string(),
            payload: json!({"test": "data"}),
            deadline_ms: 30000,
            invoked_function_arn: "arn:aws:lambda:local:000000000000:function:test".to_string(),
            trace_id: Some("trace-123".to_string()),
        },
        WebSocketMessage::Response {
            request_id: "req-123".to_string(),
            payload: json!({"result": "success"}),
            headers: Some(std::collections::HashMap::new()),
        },
        WebSocketMessage::Error {
            request_id: "req-123".to_string(),
            error_message: "Test error".to_string(),
            error_type: "TestError".to_string(),
            stack_trace: Some(vec!["line 1".to_string(), "line 2".to_string()]),
            headers: Some(std::collections::HashMap::new()),
        },
        WebSocketMessage::Ping,
        WebSocketMessage::Pong,
        WebSocketMessage::ErrorResponse {
            message: "Connection failed".to_string(),
            code: "CONNECTION_ERROR".to_string(),
        },
    ];

    for message in messages {
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();
        
        // Verify the message can be round-tripped
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }
}

#[tokio::test]
async fn test_websocket_query_parsing() {
    use lambda_runtime_api::websocket::WebSocketQuery;
    use serde_json;

    // Test query parameter parsing
    let query_json = r#"{"fn": "test-function", "rt": "nodejs18.x", "ver": "1", "eh": "abc123"}"#;
    let query: WebSocketQuery = serde_json::from_str(query_json).unwrap();
    
    assert_eq!(query.function_name, "test-function");
    assert_eq!(query.runtime, Some("nodejs18.x".to_string()));
    assert_eq!(query.version, Some("1".to_string()));
    assert_eq!(query.env_hash, Some("abc123".to_string()));

    // Test minimal query
    let minimal_json = r#"{"fn": "test-function"}"#;
    let minimal_query: WebSocketQuery = serde_json::from_str(minimal_json).unwrap();
    
    assert_eq!(minimal_query.function_name, "test-function");
    assert_eq!(minimal_query.runtime, None);
    assert_eq!(minimal_query.version, None);
    assert_eq!(minimal_query.env_hash, None);
}

#[tokio::test]
async fn test_websocket_message_roundtrip() {
    use lambda_runtime_api::websocket::WebSocketMessage;
    use serde_json;

    // Test that messages can be serialized to JSON and back
    let original = WebSocketMessage::Invocation {
        request_id: "test-request-123".to_string(),
        payload: json!({
            "key1": "value1",
            "key2": 42,
            "key3": [1, 2, 3],
            "key4": {"nested": "object"}
        }),
        deadline_ms: 30000,
        invoked_function_arn: "arn:aws:lambda:local:000000000000:function:test-function".to_string(),
        trace_id: Some("trace-123".to_string()),
    };

    // Serialize to JSON
    let json = serde_json::to_string(&original).unwrap();
    println!("Serialized message: {}", json);

    // Deserialize back
    let deserialized: WebSocketMessage = serde_json::from_str(&json).unwrap();

    // Verify the roundtrip
    match deserialized {
        WebSocketMessage::Invocation {
            request_id,
            payload,
            deadline_ms,
            invoked_function_arn,
            trace_id,
        } => {
            assert_eq!(request_id, "test-request-123");
            assert_eq!(payload["key1"], "value1");
            assert_eq!(payload["key2"], 42);
            assert_eq!(payload["key3"], json!([1, 2, 3]));
            assert_eq!(payload["key4"]["nested"], "object");
            assert_eq!(deadline_ms, 30000);
            assert_eq!(invoked_function_arn, "arn:aws:lambda:local:000000000000:function:test-function");
            assert_eq!(trace_id, Some("trace-123".to_string()));
        }
        _ => panic!("Expected Invocation message"),
    }
}
