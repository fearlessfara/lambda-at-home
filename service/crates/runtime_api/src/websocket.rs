use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use lambda_models::{RuntimeError, RuntimeResponse};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{error, info, instrument, warn};
use uuid::Uuid;
use sha2::Digest;

use crate::state::RtState;

#[derive(Debug, Deserialize, Clone)]
pub struct WebSocketQuery {
    /// function name (required)
    #[serde(rename = "fn")]
    pub function_name: String,
    /// runtime (optional)
    #[serde(rename = "rt")]
    pub runtime: Option<String>,
    /// version (optional)
    #[serde(rename = "ver")]
    pub version: Option<String>,
    /// environment hash (optional)
    #[serde(rename = "eh")]
    pub env_hash: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    #[serde(rename = "register")]
    Register {
        function_name: String,
        runtime: Option<String>,
        version: Option<String>,
        env_hash: Option<String>,
        instance_id: Option<String>,
    },
    #[serde(rename = "invocation")]
    Invocation {
        request_id: String,
        payload: serde_json::Value,
        deadline_ms: u64,
        invoked_function_arn: String,
        trace_id: Option<String>,
    },
    #[serde(rename = "response")]
    Response {
        request_id: String,
        payload: serde_json::Value,
        headers: Option<std::collections::HashMap<String, String>>,
    },
    #[serde(rename = "error")]
    Error {
        request_id: String,
        error_message: String,
        error_type: String,
        stack_trace: Option<Vec<String>>,
        headers: Option<std::collections::HashMap<String, String>>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "error_response")]
    ErrorResponse {
        message: String,
        code: String,
    },
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WebSocketQuery>,
    State(state): State<RtState>,
) -> Response {
    info!(
        "WebSocket connection request for function: {}",
        query.function_name
    );

    ws.on_upgrade(move |socket| websocket_connection(socket, query, state))
}

#[instrument(skip(socket, query, state))]
async fn websocket_connection(
    socket: WebSocket,
    query: WebSocketQuery,
    state: RtState,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WebSocketMessage>();

    // Spawn task to handle incoming WebSocket messages
    let state_clone = state.clone();
    let query_clone = query.clone();
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Err(e) = handle_websocket_message(
                        &text,
                        &query_clone,
                        &state_clone,
                        &tx_clone,
                    ).await {
                        error!("Error handling WebSocket message: {}", e);
                        let _ = tx_clone.send(WebSocketMessage::ErrorResponse {
                            message: e.to_string(),
                            code: "MESSAGE_ERROR".to_string(),
                        });
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("WebSocket connection closed");
                    break;
                }
                Ok(Message::Ping(_data)) => {
                    let _ = tx_clone.send(WebSocketMessage::Pong);
                }
                Ok(Message::Pong(_)) => {
                    // Handle pong if needed
                }
                Ok(Message::Binary(_)) => {
                    warn!("Received binary message, ignoring");
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }
    });

    // Spawn task to send messages to WebSocket
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let text = match serde_json::to_string(&msg) {
                Ok(t) => t,
                Err(e) => {
                    error!("Failed to serialize WebSocket message: {}", e);
                    continue;
                }
            };

            if sender.send(Message::Text(text)).await.is_err() {
                error!("Failed to send WebSocket message");
                break;
            }
        }
    });

    // Start the invocation loop
    if let Err(e) = start_invocation_loop(query, state, tx).await {
        error!("Invocation loop error: {}", e);
    }
}

async fn handle_websocket_message(
    text: &str,
    _query: &WebSocketQuery,
    state: &RtState,
    tx: &mpsc::UnboundedSender<WebSocketMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let msg: WebSocketMessage = serde_json::from_str(text)?;

    match msg {
        WebSocketMessage::Register { .. } => {
            info!("Container registered via WebSocket");
            // Registration is handled in the invocation loop
        }
        WebSocketMessage::Response {
            request_id,
            payload,
            headers,
        } => {
            handle_response(&request_id, payload, headers, state).await?;
        }
        WebSocketMessage::Error {
            request_id,
            error_message,
            error_type,
            stack_trace,
            headers,
        } => {
            handle_error(
                &request_id,
                error_message,
                error_type,
                stack_trace,
                headers,
                state,
            ).await?;
        }
        WebSocketMessage::Ping => {
            let _ = tx.send(WebSocketMessage::Pong);
        }
        _ => {
            warn!("Unexpected WebSocket message type");
        }
    }

    Ok(())
}

async fn handle_response(
    request_id: &str,
    payload: serde_json::Value,
    headers: Option<std::collections::HashMap<String, String>>,
    state: &RtState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(control) = &state.control {
        let rr = RuntimeResponse {
            aws_request_id: Uuid::try_parse(request_id)
                .unwrap_or_else(|_| Uuid::nil()),
            payload,
        };

        let res = control.post_response(rr, headers).await;
        match res {
            Ok(_) => info!("Response posted for request: {}", request_id),
            Err(e) => error!("Failed to post response: {}", e),
        }
    } else {
        // Fallback for tests
        let mut res = lambda_control::pending::InvocationResult::ok(
            serde_json::to_vec(&payload).unwrap_or_default(),
        );
        
        if let Some(headers) = headers {
            if let Some(version) = headers.get("X-Amz-Executed-Version") {
                res.executed_version = Some(version.clone());
            }
            if let Some(log_result) = headers.get("X-Amz-Log-Result") {
                res.log_tail_b64 = Some(log_result.clone());
            }
        }

        if state.pending.complete(request_id, res) {
            info!("Response completed for request: {}", request_id);
        } else {
            error!("Failed to complete response for request: {}", request_id);
        }
    }

    Ok(())
}

async fn handle_error(
    request_id: &str,
    error_message: String,
    error_type: String,
    stack_trace: Option<Vec<String>>,
    headers: Option<std::collections::HashMap<String, String>>,
    state: &RtState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(control) = &state.control {
        let re = RuntimeError {
            aws_request_id: Uuid::try_parse(request_id)
                .unwrap_or_else(|_| Uuid::nil()),
            error_message,
            error_type: error_type.clone(),
            stack_trace,
        };

        let mut error_headers = std::collections::HashMap::new();
        error_headers.insert("X-Amz-Function-Error".to_string(), error_type);
        if let Some(headers) = headers {
            if let Some(log_result) = headers.get("X-Amz-Log-Result") {
                error_headers.insert("X-Amz-Log-Result".to_string(), log_result.clone());
            }
        }

        let res = control.post_error(re, Some(error_headers)).await;
        match res {
            Ok(_) => info!("Error posted for request: {}", request_id),
            Err(e) => error!("Failed to post error: {}", e),
        }
    } else {
        // Fallback for tests
        let mut res = lambda_control::pending::InvocationResult::err(
            &error_type,
            error_message.as_bytes().to_vec(),
        );
        
        if let Some(headers) = headers {
            if let Some(log_result) = headers.get("X-Amz-Log-Result") {
                res.log_tail_b64 = Some(log_result.clone());
            }
        }

        if state.pending.complete(request_id, res) {
            info!("Error completed for request: {}", request_id);
        } else {
            error!("Failed to complete error for request: {}", request_id);
        }
    }

    Ok(())
}

#[instrument(skip(query, state, tx))]
async fn start_invocation_loop(
    query: WebSocketQuery,
    state: RtState,
    tx: mpsc::UnboundedSender<WebSocketMessage>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let function_name = &query.function_name;

    if let Some(control) = state.control.clone() {
        // Resolve runtime/version from control to ensure FnKey matches the queued work
        let (rt, ver, eh) = match control.get_function(function_name).await {
            Ok(f) => {
                // Compute env_hash compatible with FnKey::from_work_item
                let env_opt: Option<std::collections::HashMap<String, String>> =
                    Some(f.environment.clone());
                let env_value = serde_json::to_value(&env_opt).unwrap_or(serde_json::Value::Null);
                let stable_bytes = serde_json::to_vec(&env_value).unwrap_or_default();
                let mut hasher = sha2::Sha256::new();
                hasher.update(&stable_bytes);
                let env_hash = format!("{:x}", hasher.finalize());
                (f.runtime.clone(), Some(f.version.clone()), Some(env_hash))
            }
            Err(_) => (
                query.runtime
                    .clone()
                    .unwrap_or_else(|| "nodejs18.x".to_string()),
                query.version.clone(),
                query.env_hash.clone(),
            ),
        };

        loop {
            // Get next invocation
            match control
                .get_next_invocation(function_name, &rt, ver.as_deref(), eh.as_deref())
                .await
            {
                Ok(inv) => {
                    let msg = WebSocketMessage::Invocation {
                        request_id: inv.aws_request_id.to_string(),
                        payload: inv.payload,
                        deadline_ms: inv.deadline_ms as u64,
                        invoked_function_arn: inv.invoked_function_arn,
                        trace_id: inv.trace_id,
                    };

                    if tx.send(msg).is_err() {
                        error!("Failed to send invocation message");
                        break;
                    }
                }
                Err(e) => {
                    error!("Error getting next invocation: {}", e);
                    let _ = tx.send(WebSocketMessage::ErrorResponse {
                        message: e.to_string(),
                        code: "INVOCATION_ERROR".to_string(),
                    });
                    break;
                }
            }
        }
    } else {
        // Fallback for tests using local queues
        let key = lambda_control::queues::FnKey {
            function_name: function_name.clone(),
            runtime: query.runtime.unwrap_or_else(|| "nodejs18.x".to_string()),
            version: query.version.unwrap_or_else(|| "LATEST".to_string()),
            env_hash: query.env_hash.unwrap_or_else(|| "".to_string()),
        };

        loop {
            match state.queues.pop_or_wait(&key).await {
                Ok(work_item) => {
                    let msg = WebSocketMessage::Invocation {
                        request_id: work_item.request_id,
                        payload: serde_json::from_slice(&work_item.payload)
                            .unwrap_or(serde_json::Value::Null),
                        deadline_ms: work_item.deadline_ms as u64,
                        invoked_function_arn: format!(
                            "arn:aws:lambda:local:000000000000:function:{}",
                            key.function_name
                        ),
                        trace_id: None,
                    };

                    if tx.send(msg).is_err() {
                        error!("Failed to send invocation message");
                        break;
                    }
                }
                Err(e) => {
                    error!("Error in fallback queue pop_or_wait: {}", e);
                    let _ = tx.send(WebSocketMessage::ErrorResponse {
                        message: e.to_string(),
                        code: "QUEUE_ERROR".to_string(),
                    });
                    break;
                }
            }
        }
    }

    Ok(())
}
