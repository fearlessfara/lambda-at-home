use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info, instrument};

use crate::state::RtState;
use lambda_control::pending::InvocationResult;
use lambda_models::{RuntimeResponse, RuntimeError};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct NextQuery {
    /// function name (required)
    #[serde(rename = "fn")]
    pub function_name: String,
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
) -> impl IntoResponse {
    let function_name = &q.function_name;

    info!("Container requesting work for function: {}", function_name);

    // Prefer control plane (shared queues). Fallback to local queues in tests.
    if let Some(control) = state.control.clone() {
        // Long-lived GET: block until a work item is available.
        match control.get_next_invocation(function_name, "nodejs18.x", None, None).await {
            Ok(inv) => {
                info!(req_id = %inv.aws_request_id, "dispatching work item to container");
                json_response(StatusCode::OK, &json!({
                    "requestId": inv.aws_request_id.to_string(),
                    "deadlineMs": inv.deadline_ms,
                    "event": inv.payload,
                }))
            }
            Err(e) => {
                error!(error=?e, "Error getting next invocation");
                json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}))
            }
        }
    } else {
        // Test fallback: block until a work item is available
        match state.queues.pop_or_wait(function_name).await {
            Ok(work_item) => {
                info!(req_id = %work_item.request_id, "dispatching work item to container (fallback)");
                json_response(StatusCode::OK, &json!({
                    "requestId": work_item.request_id,
                    "deadlineMs": work_item.deadline_ms,
                    "event": serde_json::from_slice::<serde_json::Value>(&work_item.payload).unwrap_or(serde_json::Value::Null),
                }))
            }
            Err(e) => {
                error!(error=?e, "Error in fallback queue pop_or_wait");
                json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}))
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
        match control.post_response(rr, hdrs).await {
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
        match control.post_error(re, hdrs).await {
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
