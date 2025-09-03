use serde::{Deserialize, Serialize};
use lambda_models::{Function, InvokeRequest};

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FunctionMeta {
    pub function_name: String,
    pub runtime: String,
    pub version: Option<String>,
    pub environment: Option<std::collections::HashMap<String, String>>,
    pub timeout_ms: u32,
}

impl From<Function> for FunctionMeta {
    fn from(func: Function) -> Self {
        Self {
            function_name: func.function_name,
            runtime: func.runtime,
            version: Some(func.version),
            environment: Some(func.environment),
            timeout_ms: func.timeout as u32,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorkItem {
    pub request_id: String,         // opaque string (UUID v4)
    pub function: FunctionMeta,     // used to derive FnKey
    pub payload: Vec<u8>,
    pub deadline_ms: i64,           // wall clock deadline used by RIC
    pub log_type: Option<String>,   // "Tail" | "None"
    pub client_context: Option<String>,
    pub cognito_identity: Option<String>,
}

impl WorkItem {
    /// Create a WorkItem from an InvokeRequest and Function
    pub fn from_invoke_request(req_id: String, function: Function, request: InvokeRequest) -> Self {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        
        let deadline_ms = now_ms + (function.timeout * 1000) as i64;
        
        let payload = if let Some(payload) = request.payload {
            serde_json::to_vec(&payload).unwrap_or_default()
        } else {
            Vec::new()
        };
        
        Self {
            request_id: req_id,
            function: FunctionMeta::from(function),
            payload,
            deadline_ms,
            log_type: request.log_type.map(|lt| format!("{:?}", lt)),
            client_context: request.client_context,
            cognito_identity: None, // TODO: Extract from request if available
        }
    }
}