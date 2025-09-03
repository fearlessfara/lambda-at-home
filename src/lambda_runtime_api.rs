use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaInvocation {
    pub request_id: String,
    pub function_id: Uuid,
    pub payload: Value,
    pub deadline_ms: u64,
    pub invoked_function_arn: String,
    pub trace_id: Option<String>,
    pub client_context: Option<Value>,
    pub cognito_identity: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaResponse {
    pub request_id: String,
    pub response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaError {
    pub request_id: String,
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitError {
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<Vec<String>>,
}

pub struct LambdaRuntimeService {
    // Pending invocations waiting for RIC to poll (FIFO queue)
    pending_invocations: Arc<RwLock<VecDeque<LambdaInvocation>>>,
    // Completed responses from RIC
    completed_responses: Arc<RwLock<HashMap<String, LambdaResponse>>>,
    // Error responses from RIC
    error_responses: Arc<RwLock<HashMap<String, LambdaError>>>,
    // Container ID to function ID mapping
    container_functions: Arc<RwLock<HashMap<String, Uuid>>>,
    // Active invocations per function (for concurrency control)
    active_invocations: Arc<RwLock<HashMap<Uuid, usize>>>,
    // Request ID to function ID mapping (for proper concurrency tracking)
    request_to_function: Arc<RwLock<HashMap<String, Uuid>>>,
    // Maximum concurrent invocations per function
    max_concurrent_invocations: usize,
}

impl LambdaRuntimeService {
    pub fn new() -> Self {
        Self {
            pending_invocations: Arc::new(RwLock::new(VecDeque::new())),
            completed_responses: Arc::new(RwLock::new(HashMap::new())),
            error_responses: Arc::new(RwLock::new(HashMap::new())),
            container_functions: Arc::new(RwLock::new(HashMap::new())),
            active_invocations: Arc::new(RwLock::new(HashMap::new())),
            request_to_function: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_invocations: 1000, // Default AWS Lambda limit
        }
    }

    pub fn with_max_concurrency(max_concurrent: usize) -> Self {
        Self {
            pending_invocations: Arc::new(RwLock::new(VecDeque::new())),
            completed_responses: Arc::new(RwLock::new(HashMap::new())),
            error_responses: Arc::new(RwLock::new(HashMap::new())),
            container_functions: Arc::new(RwLock::new(HashMap::new())),
            active_invocations: Arc::new(RwLock::new(HashMap::new())),
            request_to_function: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_invocations: max_concurrent,
        }
    }

    /// Register a container with a function ID
    pub async fn register_container(&self, container_id: String, function_id: Uuid) {
        let mut mapping = self.container_functions.write().await;
        mapping.insert(container_id.clone(), function_id);
        info!("Registered container {} with function {}", container_id, function_id);
    }

    /// Queue an invocation for a function
    pub async fn queue_invocation(&self, function_id: Uuid, payload: Value) -> Result<String> {
        // Check concurrency limits - only count active invocations (not pending)
        let current_active = {
            let active = self.active_invocations.read().await;
            active.get(&function_id).copied().unwrap_or(0)
        };

        if current_active >= self.max_concurrent_invocations {
            return Err(anyhow::anyhow!("Function {} has reached maximum concurrent invocations ({})", function_id, self.max_concurrent_invocations));
        }

        let request_id = Uuid::new_v4().to_string();
        let invocation = LambdaInvocation {
            request_id: request_id.clone(),
            function_id,
            payload,
            deadline_ms: chrono::Utc::now().timestamp_millis() as u64 + 300000, // 5 minutes from now
            invoked_function_arn: format!("arn:aws:lambda:us-east-1:123456789012:function:{}", function_id),
            trace_id: Some(format!("Root=1-{}-{}", 
                chrono::Utc::now().format("%Y%m%d%H%M%S"),
                Uuid::new_v4().to_string()[..8].to_string()
            )),
            client_context: None,
            cognito_identity: None,
        };

        let mut pending = self.pending_invocations.write().await;
        pending.push_back(invocation);
        
        // Store request-to-function mapping for concurrency tracking
        let mut request_mapping = self.request_to_function.write().await;
        request_mapping.insert(request_id.clone(), function_id);
        
        info!("Queued invocation {} for function {} (active: {}/{})", request_id, function_id, current_active, self.max_concurrent_invocations);
        Ok(request_id)
    }

    /// Get next invocation for a container (RIC polling)
    pub async fn get_next_invocation(&self, container_id: &str) -> Result<Option<LambdaInvocation>> {
        // Find the function ID for this container
        let function_id = {
            let mapping = self.container_functions.read().await;
            mapping.get(container_id).copied()
        };

        if let Some(function_id) = function_id {
            // Find pending invocation for this function
            let mut pending = self.pending_invocations.write().await;
            
            // Find the first invocation for this function (FIFO order)
            if let Some(pos) = pending.iter().position(|inv| inv.function_id == function_id) {
                let invocation = pending.remove(pos).unwrap();
                
                // Increment active invocation count
                let mut active = self.active_invocations.write().await;
                let current_count = active.get(&function_id).copied().unwrap_or(0);
                active.insert(function_id, current_count + 1);
                
                info!("RIC {} polling: returning invocation {} for function {} (active: {})", container_id, invocation.request_id, function_id, current_count + 1);
                return Ok(Some(invocation));
            }
        }

        // No pending invocations for this container's function
        Ok(None)
    }

    /// Submit response from RIC
    pub async fn submit_response(&self, request_id: String, response: Value) -> Result<()> {
        let lambda_response = LambdaResponse {
            request_id: request_id.clone(),
            response,
        };

        let mut completed = self.completed_responses.write().await;
        completed.insert(request_id.clone(), lambda_response);
        
        // Decrement active invocation count
        self.decrement_active_invocation(&request_id).await;
        
        info!("Received response for invocation {}", request_id);
        Ok(())
    }

    /// Submit error from RIC
    pub async fn submit_error(&self, request_id: String, error_type: String, error_message: String, stack_trace: Option<Vec<String>>) -> Result<()> {
        let lambda_error = LambdaError {
            request_id: request_id.clone(),
            error_type,
            error_message,
            stack_trace,
        };

        let mut errors = self.error_responses.write().await;
        errors.insert(request_id.clone(), lambda_error);
        
        // Decrement active invocation count
        self.decrement_active_invocation(&request_id).await;
        
        info!("Received error for invocation {}", request_id);
        Ok(())
    }

    /// Get response for an invocation
    pub async fn get_response(&self, request_id: &str) -> Result<Option<LambdaResponse>> {
        let completed = self.completed_responses.read().await;
        Ok(completed.get(request_id).cloned())
    }

    /// Get error for an invocation
    pub async fn get_error(&self, request_id: &str) -> Result<Option<LambdaError>> {
        let errors = self.error_responses.read().await;
        Ok(errors.get(request_id).cloned())
    }

    /// Decrement active invocation count for a function
    async fn decrement_active_invocation(&self, request_id: &str) {
        // Find the function ID for this request
        let function_id = {
            let request_mapping = self.request_to_function.read().await;
            request_mapping.get(request_id).copied()
        };

        if let Some(function_id) = function_id {
            let mut active = self.active_invocations.write().await;
            if let Some(count) = active.get_mut(&function_id) {
                if *count > 0 {
                    *count -= 1;
                    info!("Decremented active invocation count for function {} to {} (request: {})", function_id, count, request_id);
                }
            }
            
            // Clean up the request mapping
            let mut request_mapping = self.request_to_function.write().await;
            request_mapping.remove(request_id);
        }
    }

    /// Create the Lambda Runtime API router
    pub fn create_router(&self) -> Router<Arc<Self>> {
        Router::new()
            // Lambda Runtime API endpoints
            .route("/runtime/invocation/next", get(get_next_invocation))
            .route("/runtime/invocation/:request_id/response", post(submit_response))
            .route("/runtime/invocation/:request_id/error", post(submit_error))
            .route("/runtime/init/error", get(get_init_error))
            .route("/runtime/init/error", post(submit_init_error))
            // Direct execution endpoint for testing
            .route("/execute", post(execute_function))
            // Health check endpoint
            .route("/health", get(health_check))
    }
}

/// GET /runtime/invocation/next - RIC polls for next invocation
async fn get_next_invocation(
    State(service): State<Arc<LambdaRuntimeService>>,
    headers: HeaderMap,
) -> Result<Json<LambdaInvocation>, StatusCode> {
    // Extract container ID from headers (Lambda sets this)
    let container_id = headers
        .get("lambda-runtime-aws-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    match service.get_next_invocation(container_id).await {
        Ok(Some(invocation)) => {
            info!("RIC polling: returning invocation {} to container {}", invocation.request_id, container_id);
            Ok(Json(invocation))
        }
        Ok(None) => {
            // No pending invocations - this should block in real Lambda
            // For now, return 204 No Content
            Err(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            warn!("Error getting next invocation: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /runtime/invocation/{request_id}/response - RIC submits response
async fn submit_response(
    State(service): State<Arc<LambdaRuntimeService>>,
    Path(request_id): Path<String>,
    Json(response): Json<Value>,
) -> Result<StatusCode, StatusCode> {
    match service.submit_response(request_id.clone(), response).await {
        Ok(_) => {
            info!("Received response for invocation {}", request_id);
            Ok(StatusCode::ACCEPTED)
        }
        Err(e) => {
            warn!("Error submitting response for {}: {}", request_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /runtime/invocation/{request_id}/error - RIC submits error
async fn submit_error(
    State(service): State<Arc<LambdaRuntimeService>>,
    Path(request_id): Path<String>,
    Json(error): Json<LambdaError>,
) -> Result<StatusCode, StatusCode> {
    match service.submit_error(request_id.clone(), error.error_type, error.error_message, error.stack_trace).await {
        Ok(_) => {
            info!("Received error for invocation {}", request_id);
            Ok(StatusCode::ACCEPTED)
        }
        Err(e) => {
            warn!("Error submitting error for {}: {}", request_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /runtime/init/error - RIC gets initialization error
async fn get_init_error(
    State(_service): State<Arc<LambdaRuntimeService>>,
) -> Result<Json<InitError>, StatusCode> {
    // For now, no initialization errors
    Err(StatusCode::NO_CONTENT)
}

/// POST /runtime/init/error - RIC submits initialization error
async fn submit_init_error(
    State(_service): State<Arc<LambdaRuntimeService>>,
    Json(_error): Json<InitError>,
) -> Result<StatusCode, StatusCode> {
    // For now, just accept the error
    Ok(StatusCode::ACCEPTED)
}

/// POST /execute - Direct function execution endpoint for testing
async fn execute_function(
    State(service): State<Arc<LambdaRuntimeService>>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Extract function_id from payload
    let function_id = match payload.get("function_id") {
        Some(id) => {
            match id.as_str() {
                Some(id_str) => {
                    match uuid::Uuid::parse_str(id_str) {
                        Ok(uuid) => uuid,
                        Err(_) => {
                            return Err(StatusCode::BAD_REQUEST);
                        }
                    }
                }
                None => {
                    return Err(StatusCode::BAD_REQUEST);
                }
            }
        }
        None => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Extract event payload
    let event_payload = payload.get("event").cloned().unwrap_or(json!({}));

    // Queue the invocation
    match service.queue_invocation(function_id, event_payload.clone()).await {
        Ok(request_id) => {
            // For direct execution, we'll simulate the execution
            // In a real implementation, this would wait for the RIC to process it
            // For testing purposes, we'll return a mock response
            let response = json!({
                "statusCode": 200,
                "body": json!({
                    "message": "Function executed successfully",
                    "requestId": request_id,
                    "functionId": function_id.to_string(),
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "event": event_payload
                }),
                "headers": {
                    "Content-Type": "application/json",
                    "X-Request-ID": request_id
                }
            });

            Ok(Json(response))
        }
        Err(e) => {
            warn!("Error executing function {}: {}", function_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// GET /health - Health check endpoint
async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}
