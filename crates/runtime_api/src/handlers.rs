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
use lambda_control::queues::FnKey;
use lambda_control::pending::InvocationResult;

#[derive(Deserialize, Debug)]
pub struct NextQuery {
    /// optional, if you pin to a specific function name
    #[serde(rename = "fn")]
    pub function_name: Option<String>,
    /// runtime identifier (e.g., "nodejs18.x", "python3.11")
    #[serde(rename = "rt")]
    pub runtime: String,
    /// version string; default "LATEST"
    #[serde(rename = "ver")]
    pub version: Option<String>,
    /// env hash (must match FnKey algo)
    #[serde(rename = "eh")]
    pub env_hash: String,
}

fn json_response<T: serde::Serialize>(status: StatusCode, v: &T) -> Response {
    let mut res = Response::new(Body::from(serde_json::to_vec(v).unwrap_or_default()));
    *res.status_mut() = status;
    res.headers_mut()
        .insert(axum::http::header::CONTENT_TYPE, HeaderValue::from_static("application/json"));
    res
}

#[instrument(skip(state), fields(rt = %q.runtime, ver = %q.version.as_deref().unwrap_or("LATEST"), env = %q.env_hash, func = %q.function_name.as_deref().unwrap_or("-")))]
pub async fn runtime_next(
    State(state): State<RtState>,
    Query(q): Query<NextQuery>,
) -> impl IntoResponse {
    let key = FnKey {
        function_name: q.function_name.unwrap_or_default(),
        runtime: q.runtime,
        version: q.version.unwrap_or_else(|| "LATEST".to_string()),
        env_hash: q.env_hash,
    };

    // Block until work is available (lost-wakeup safe inside queues.pop_or_wait)
    match state.queues.pop_or_wait(&key).await {
        Ok(wi) => {
            info!(req_id = %wi.request_id, "dispatching work item to container");
            json_response(StatusCode::OK, &json!({
                "requestId": wi.request_id,
                "deadlineMs": wi.deadline_ms,
                "event": serde_json::from_slice::<serde_json::Value>(&wi.payload).unwrap_or(serde_json::Value::Null),
                // minimal context; add fields if you expose them
            }))
        }
        Err(e) => {
            error!(error = ?e, "failed to get next work item");
            json_response(StatusCode::INTERNAL_SERVER_ERROR, &json!({"error": e.to_string()}))
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
    // Build result (ok)
    let mut res = InvocationResult::ok(body.to_vec());
    // Optional Lambda headers
    if let Some(v) = headers.get("X-Amz-Executed-Version").and_then(|h| h.to_str().ok()) {
        res.executed_version = Some(v.to_string());
    }
    if let Some(v) = headers.get("X-Amz-Log-Result").and_then(|h| h.to_str().ok()) {
        res.log_tail_b64 = Some(v.to_string());
    }

    let delivered = state.pending.complete(&request_id, res);
    if delivered {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    }
}

#[instrument(skip(state, body, headers), fields(req_id = %request_id))]
pub async fn runtime_error(
    Path(request_id): Path<String>,
    State(state): State<RtState>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> impl IntoResponse {
    // Lambda semantics: error kind header; default Unhandled
    let kind = headers
        .get("X-Amz-Function-Error")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unhandled");

    let mut res = InvocationResult::err(kind, body.to_vec());
    if let Some(v) = headers.get("X-Amz-Log-Result").and_then(|h| h.to_str().ok()) {
        res.log_tail_b64 = Some(v.to_string());
    }

    let delivered = state.pending.complete(&request_id, res);
    if delivered {
        StatusCode::ACCEPTED
    } else {
        StatusCode::NOT_FOUND
    }
}

pub async fn runtime_healthz() -> &'static str {
    "ok"
}
