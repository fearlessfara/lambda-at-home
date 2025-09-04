use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, HeaderName, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info, instrument};
use sha2::{Digest, Sha256};

use crate::state::RtState;
use lambda_control::pending::InvocationResult;
use lambda_control::queues::FnKey;
use lambda_models::{RuntimeResponse, RuntimeError};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct NextQuery {
    /// function name (required)
    #[serde(rename = "fn")]
    pub function_name: String,
    /// runtime (optional in handler, defaults maintained for back-compat)
    #[serde(rename = "rt")]
    pub runtime: Option<String>,
    /// version (optional)
    #[serde(rename = "ver")]
    pub version: Option<String>,
    /// environment hash (optional)
    #[serde(rename = "eh")]
    pub env_hash: Option<String>,
}

fn json_response<T: serde::Serialize>(status: StatusCode, v: &T) -> Response {
    let mut res = Response::new(Body::from(serde_json::to_vec(v).unwrap_or_default()));
    *res.status_mut() = status;
    res.headers_mut()
        .insert(axum::http::header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    res
}

#[instrument(skip(state), fields(func = %q.function_name))]
pub async fn runtime_next(
    State(state): State<RtState>,
    Query(q): Query<NextQuery>,
    headers_in: HeaderMap,
) -> impl IntoResponse {
    let function_name = &q.function_name;

    info!("Container requesting work for function: {}", function_name);

    // Prefer control plane (shared queues). Fallback to local queues in tests.
    if let Some(control) = state.control.clone() {
        // Resolve runtime/version from control to ensure FnKey matches the queued work
        let (rt, ver, eh) = match control.get_function(function_name).await {
            Ok(f) => {
                // Compute env_hash compatible with FnKey::from_work_item (Some(environment))
                let env_opt: Option<std::collections::HashMap<String, String>> = Some(f.environment.clone());
                let env_value = serde_json::to_value(&env_opt).unwrap_or(serde_json::Value::Null);
                let stable_bytes = serde_json::to_vec(&env_value).unwrap_or_default();
                let mut hasher = Sha256::new();
                hasher.update(&stable_bytes);
                let env_hash = format!("{:x}", hasher.finalize());
                (f.runtime.clone(), Some(f.version.clone()), Some(env_hash))
            },
            Err(_) => (q.runtime.clone().unwrap_or_else(|| "nodejs18.x".to_string()), q.version.clone(), q.env_hash.clone()),
        };
        // Long-lived GET: block until a work item is available.
        match control.get_next_invocation(function_name, &rt, ver.as_deref(), eh.as_deref()).await {
            Ok(inv) => {
                info!(req_id = %inv.aws_request_id, "dispatching work item to container");
                // Mark instance active using container-provided instance ID
                if let Some(inst_id) = headers_in.get("x-lambdah-instance-id").and_then(|v| v.to_str().ok()) {
                    let _ = control.mark_instance_active_by_id(inst_id).await;
                }
                // Build AWS Lambda style headers and body as the event JSON
                let mut res = Response::new(Body::from(serde_json::to_vec(&inv.payload).unwrap_or_default()));
                *res.status_mut() = StatusCode::OK;
                let headers = res.headers_mut();
                headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("application/json"));
                headers.insert(HeaderName::from_static("lambda-runtime-aws-request-id"), HeaderValue::from_str(&inv.aws_request_id.to_string()).unwrap());
                headers.insert(HeaderName::from_static("lambda-runtime-deadline-ms"), HeaderValue::from_str(&inv.deadline_ms.to_string()).unwrap());
                headers.insert(HeaderName::from_static("lambda-runtime-invoked-function-arn"), HeaderValue::from_str(&inv.invoked_function_arn).unwrap());
                if let Some(tid) = &inv.trace_id { if let Ok(hv) = HeaderValue::from_str(tid) { headers.insert(HeaderName::from_static("lambda-runtime-trace-id"), hv); } }
                return res;
            }
            Err(e) => {
                error!(error=?e, "Error getting next invocation");
                return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
            }
        }
    } else {
        // Test fallback: block until a work item is available using FnKey
        let key = FnKey {
            function_name: function_name.clone(),
            runtime: q.runtime.unwrap_or_else(|| "nodejs18.x".to_string()),
            version: q.version.unwrap_or_else(|| "LATEST".to_string()),
            env_hash: q.env_hash.unwrap_or_else(|| "".to_string()),
        };
        match state.queues.pop_or_wait(&key).await {
            Ok(work_item) => {
                info!(req_id = %work_item.request_id, "dispatching work item to container (fallback)");
                let mut res = Response::new(Body::from(work_item.payload.clone()));
                *res.status_mut() = StatusCode::OK;
                let headers = res.headers_mut();
                headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("application/json"));
                headers.insert(HeaderName::from_static("lambda-runtime-aws-request-id"), HeaderValue::from_str(&work_item.request_id).unwrap());
                headers.insert(HeaderName::from_static("lambda-runtime-deadline-ms"), HeaderValue::from_str(&work_item.deadline_ms.to_string()).unwrap());
                headers.insert(HeaderName::from_static("lambda-runtime-invoked-function-arn"), HeaderValue::from_str(&format!("arn:aws:lambda:local:000000000000:function:{}", key.function_name)).unwrap());
                return res;
            }
            Err(e) => {
                error!(error=?e, "Error in fallback queue pop_or_wait");
                return json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}));
            }
        }
    }
}

#[instrument(skip(state, body, headers), fields(req_id = %request_id))]
pub async fn runtime_response(
    Path(request_id): Path<String>,
    State(state): State<RtState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    if let Some(control) = state.control.clone() {
        // Route through control plane so pending waiter is shared
        let payload_json = serde_json::from_slice::<serde_json::Value>(&body).unwrap_or(serde_json::Value::Null);
        let rr = RuntimeResponse { aws_request_id: Uuid::try_parse(&request_id).unwrap_or_else(|_| Uuid::nil()), payload: payload_json };
        let hdrs = {
            let mut map = std::collections::HashMap::new();
            if let Some(v) = headers.get("X-Amz-Executed-Version").and_then(|h| h.to_str().ok()) { map.insert("X-Amz-Executed-Version".to_string(), v.to_string()); }
            if let Some(v) = headers.get("X-Amz-Log-Result").and_then(|h| h.to_str().ok()) { map.insert("X-Amz-Log-Result".to_string(), v.to_string()); }
            if map.is_empty() { None } else { Some(map) }
        };
        let res = control.post_response(rr, hdrs).await;
        // Mark instance idle again
        if let Some(inst_id) = headers.get("x-lambdah-instance-id").and_then(|v| v.to_str().ok()) {
            let _ = control.mark_instance_idle_by_id(inst_id).await;
        }
        match res {
            Ok(_) => StatusCode::ACCEPTED,
            Err(_) => StatusCode::NOT_FOUND,
        }
    } else {
        // Fallback for tests using local Pending
        let mut res = InvocationResult::ok(body.to_vec());
        if let Some(v) = headers.get("X-Amz-Executed-Version").and_then(|h| h.to_str().ok()) { res.executed_version = Some(v.to_string()); }
        if let Some(v) = headers.get("X-Amz-Log-Result").and_then(|h| h.to_str().ok()) { res.log_tail_b64 = Some(v.to_string()); }
        if state.pending.complete(&request_id, res) { StatusCode::ACCEPTED } else { StatusCode::NOT_FOUND }
    }
}

#[instrument(skip(state, body, headers), fields(req_id = %request_id))]
pub async fn runtime_error(
    Path(request_id): Path<String>,
    State(state): State<RtState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    if let Some(control) = state.control.clone() {
        let err_kind = headers.get("X-Amz-Function-Error").and_then(|h| h.to_str().ok()).unwrap_or("Unhandled").to_string();
        let payload_json = serde_json::from_slice::<serde_json::Value>(&body).unwrap_or(serde_json::Value::Null);
        let re = RuntimeError { aws_request_id: Uuid::try_parse(&request_id).unwrap_or_else(|_| Uuid::nil()), error_message: payload_json.to_string(), error_type: err_kind.clone(), stack_trace: None };
        let hdrs = {
            let mut map = std::collections::HashMap::new();
            map.insert("X-Amz-Function-Error".to_string(), err_kind);
            if let Some(v) = headers.get("X-Amz-Log-Result").and_then(|h| h.to_str().ok()) { map.insert("X-Amz-Log-Result".to_string(), v.to_string()); }
            Some(map)
        };
        let res = control.post_error(re, hdrs).await;
        // Mark instance idle again on error
        if let Some(inst_id) = headers.get("x-lambdah-instance-id").and_then(|v| v.to_str().ok()) {
            let _ = control.mark_instance_idle_by_id(inst_id).await;
        }
        match res {
            Ok(_) => StatusCode::ACCEPTED,
            Err(_) => StatusCode::NOT_FOUND,
        }
    } else {
        let kind = headers.get("X-Amz-Function-Error").and_then(|h| h.to_str().ok()).unwrap_or("Unhandled");
        let mut res = InvocationResult::err(kind, body.to_vec());
        if let Some(v) = headers.get("X-Amz-Log-Result").and_then(|h| h.to_str().ok()) { res.log_tail_b64 = Some(v.to_string()); }
        if state.pending.complete(&request_id, res) { StatusCode::ACCEPTED } else { StatusCode::NOT_FOUND }
    }
}

pub async fn runtime_healthz() -> &'static str {
    "ok"
}
