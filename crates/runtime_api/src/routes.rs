use axum::{routing::{get, post}, Router};
use crate::handlers::*;

pub fn build_router(state: crate::state::RtState) -> Router {
    Router::new()
        .route("/runtime/healthz", get(runtime_healthz))
        .route("/2018-06-01/runtime/invocation/next", get(runtime_next))
        .route("/2018-06-01/runtime/invocation/:request_id/response", post(runtime_response))
        .route("/2018-06-01/runtime/invocation/:request_id/error", post(runtime_error))
        .with_state(state)
}
