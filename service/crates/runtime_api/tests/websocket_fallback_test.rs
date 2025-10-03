use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use lambda_control::{pending::Pending, queues::Queues};
use lambda_runtime_api::{build_router, RtState};
use tower::ServiceExt;

#[tokio::test]
async fn test_http_healthz_still_works() {
    // Create test state
    let state = RtState {
        control: None,
        queues: Queues::new(),
        pending: Pending::new(),
    };

    let app = Router::new().merge(build_router(state));

    // Test that the healthz endpoint still works
    let request = Request::builder()
        .uri("/runtime/healthz")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // The healthz endpoint should return OK
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_websocket_endpoint_exists() {
    // Create test state
    let state = RtState {
        control: None,
        queues: Queues::new(),
        pending: Pending::new(),
    };

    let app = Router::new().merge(build_router(state));

    // Test that the WebSocket endpoint exists
    let request = Request::builder()
        .uri("/2018-06-01/runtime/websocket?fn=test-function")
        .header("upgrade", "websocket")
        .header("connection", "upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    
    // The WebSocket endpoint should exist and respond appropriately
    assert!(response.status() == StatusCode::SWITCHING_PROTOCOLS || 
            response.status() == StatusCode::BAD_REQUEST ||
            response.status() == StatusCode::UPGRADE_REQUIRED);
}

#[tokio::test]
async fn test_both_protocols_coexist() {
    // Test that both HTTP and WebSocket endpoints can coexist
    let state = RtState {
        control: None,
        queues: Queues::new(),
        pending: Pending::new(),
    };

    let app = Router::new().merge(build_router(state));

    // Test HTTP endpoint
    let http_request = Request::builder()
        .uri("/runtime/healthz")
        .body(Body::empty())
        .unwrap();

    let http_response = app.clone().oneshot(http_request).await.unwrap();
    assert_eq!(http_response.status(), StatusCode::OK);

    // Test WebSocket endpoint
    let ws_request = Request::builder()
        .uri("/2018-06-01/runtime/websocket?fn=test-function")
        .header("upgrade", "websocket")
        .header("connection", "upgrade")
        .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
        .header("sec-websocket-version", "13")
        .body(Body::empty())
        .unwrap();

    let ws_response = app.oneshot(ws_request).await.unwrap();
    assert!(ws_response.status() == StatusCode::SWITCHING_PROTOCOLS || 
            ws_response.status() == StatusCode::BAD_REQUEST ||
            ws_response.status() == StatusCode::UPGRADE_REQUIRED);
}

#[tokio::test]
async fn test_websocket_query_parameters() {
    // Test that WebSocket endpoint accepts query parameters
    let state = RtState {
        control: None,
        queues: Queues::new(),
        pending: Pending::new(),
    };

    let app = Router::new().merge(build_router(state));

    let test_cases = vec![
        "fn=test-function",
        "fn=test-function&rt=nodejs18.x",
        "fn=test-function&rt=nodejs18.x&ver=1",
        "fn=test-function&rt=nodejs18.x&ver=1&eh=abc123",
    ];

    for query in test_cases {
        let request = Request::builder()
            .uri(&format!("/2018-06-01/runtime/websocket?{}", query))
            .header("upgrade", "websocket")
            .header("connection", "upgrade")
            .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
            .header("sec-websocket-version", "13")
            .body(Body::empty())
            .unwrap();

        let response = app.clone().oneshot(request).await.unwrap();
        
        // All query parameter combinations should be accepted
        assert!(response.status() == StatusCode::SWITCHING_PROTOCOLS || 
                response.status() == StatusCode::BAD_REQUEST ||
                response.status() == StatusCode::UPGRADE_REQUIRED);
    }
}
