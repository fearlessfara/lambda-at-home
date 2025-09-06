use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use bytes::Bytes;
use lambda_runtime_api::build_router;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tower::util::ServiceExt;

// Bring control-plane types
use lambda_control::pending::Pending;
use lambda_control::queues::{FnKey, Queues};
use lambda_control::work_item::{FunctionMeta, WorkItem};
use lambda_runtime_api::state::RtState;

// Small helper: spin up the router in-memory
fn test_router() -> Router {
    let queues = Queues::new();
    let pending = Pending::new();
    build_router(RtState {
        control: None,
        queues,
        pending,
    })
}

// Build a matching FnKey for the queue (must match your container query)
fn fn_key() -> FnKey {
    // Generate the same env_hash that FnKey::from_work_item would generate for None environment
    let env_value = serde_json::to_value(&None::<serde_json::Value>).unwrap();
    let stable_bytes = serde_json::to_vec(&env_value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&stable_bytes);
    let env_hash = format!("{:x}", hasher.finalize());

    FnKey {
        function_name: "hello".to_string(),
        runtime: "nodejs18.x".to_string(),
        version: "LATEST".to_string(),
        env_hash,
    }
}

fn sample_function_meta() -> FunctionMeta {
    FunctionMeta {
        function_name: "hello".to_string(),
        runtime: "nodejs18.x".to_string(),
        version: None,
        environment: None,
        timeout_ms: 2000,
    }
}

fn work_item(req_id: &str) -> WorkItem {
    WorkItem {
        request_id: req_id.to_string(),
        function: sample_function_meta(),
        payload: br#"{"ping":"pong"}"#.to_vec(),
        deadline_ms: (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64)
            + 2000,
        log_type: None,
        client_context: None,
        cognito_identity: None,
    }
}

#[tokio::test]
async fn healthz_ok() {
    let app = test_router();
    let res = app
        .oneshot(
            Request::get("/runtime/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn next_blocks_then_returns_when_pushed() {
    let queues = Queues::new();
    let pending = Pending::new();
    let state = RtState {
        control: None,
        queues: queues.clone(),
        pending,
    };
    let app = build_router(state);

    // Start /next long-poll in background
    let app_clone = app.clone();
    let correct_env_hash = fn_key().env_hash.clone();
    let fut: JoinHandle<_> = tokio::spawn(async move {
        let uri = format!(
            "/2018-06-01/runtime/invocation/next?fn=hello&rt=nodejs18.x&ver=LATEST&eh={correct_env_hash}"
        );
        app_clone
            .oneshot(Request::get(uri).body(Body::empty()).unwrap())
            .await
            .unwrap()
    });

    // Give the handler time to enter pop_or_wait
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Push a matching WorkItem
    queues.push(work_item("req-1")).unwrap();

    // The long-poll should now complete
    let res = timeout(Duration::from_secs(2), fut).await.unwrap().unwrap();
    let headers = res.headers();
    assert_eq!(
        headers.get("lambda-runtime-aws-request-id").unwrap(),
        "req-1"
    );
    let deadline_ms = headers
        .get("lambda-runtime-deadline-ms")
        .unwrap()
        .to_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    assert!(deadline_ms > 0);
    // Body is the raw event JSON
    let bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    let event: JsonValue = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(event["ping"], "pong");
}

#[tokio::test]
async fn next_returns_json_200() {
    let queues = Queues::new();
    let pending = Pending::new();
    let state = RtState {
        control: None,
        queues: queues.clone(),
        pending,
    };
    let app = build_router(state);

    // Push before calling /next
    queues.push(work_item("req-2")).unwrap();

    let correct_env_hash = fn_key().env_hash.clone();
    let uri = format!(
        "/2018-06-01/runtime/invocation/next?fn=hello&rt=nodejs18.x&ver=LATEST&eh={correct_env_hash}"
    );
    let res = app
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let ct = res
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.starts_with("application/json"));

    // Headers should include request id and content-type
    assert_eq!(
        res.headers().get("lambda-runtime-aws-request-id").unwrap(),
        "req-2"
    );
    let bytes = axum::body::to_bytes(res.into_body(), 1024).await.unwrap();
    let event: JsonValue = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(event["ping"], "pong");
}

#[tokio::test]
async fn runtime_response_delivers_and_returns_202() {
    let queues = Queues::new();
    let pending = Pending::new();
    let state = RtState {
        control: None,
        queues,
        pending: pending.clone(),
    };
    let app = build_router(state);

    // Register a waiter (simulate invoke side)
    let req_id = "req-3".to_string();
    let rx = pending.register(req_id.clone());

    // Call runtime_response
    let body = Body::from(Bytes::from_static(br#"{"ok":true}"#));
    let req = Request::post(format!("/2018-06-01/runtime/invocation/{req_id}/response"))
        .body(body)
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);

    // The waiter should receive the result
    let delivered = timeout(Duration::from_secs(1), rx).await.unwrap().unwrap();
    assert!(delivered.ok);
    assert_eq!(delivered.payload, br#"{"ok":true}"#.to_vec());
}

#[tokio::test]
async fn runtime_error_delivers_with_kind_and_returns_202() {
    let queues = Queues::new();
    let pending = Pending::new();
    let state = RtState {
        control: None,
        queues,
        pending: pending.clone(),
    };
    let app = build_router(state);

    let req_id = "req-4".to_string();
    let rx = pending.register(req_id.clone());

    // Build request with X-Amz-Function-Error header
    let mut req = Request::post(format!("/2018-06-01/runtime/invocation/{req_id}/error"))
        .body(Body::from(Bytes::from_static(br#"{"boom":true}"#)))
        .unwrap();
    req.headers_mut()
        .insert("X-Amz-Function-Error", "Unhandled".parse().unwrap());

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::ACCEPTED);

    let delivered = timeout(Duration::from_secs(1), rx).await.unwrap().unwrap();
    assert!(!delivered.ok);
    assert_eq!(delivered.function_error.as_deref(), Some("Unhandled"));
    assert_eq!(delivered.payload, br#"{"boom":true}"#.to_vec());
}

#[tokio::test]
async fn runtime_response_404_if_no_waiter() {
    let queues = Queues::new();
    let pending = Pending::new();
    let state = RtState {
        control: None,
        queues,
        pending,
    };
    let app = build_router(state);
    // No pending.register for req-5

    let body = Body::from(Bytes::from_static(b"ignored"));
    let req = Request::post("/2018-06-01/runtime/invocation/req-5/response")
        .body(body)
        .unwrap();

    let res = app.oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
