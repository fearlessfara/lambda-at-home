use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeRequest {
    pub function_name: String,
    pub invocation_type: InvocationType,
    pub log_type: Option<LogType>,
    pub client_context: Option<String>, // base64 encoded
    pub payload: Option<serde_json::Value>,
    pub qualifier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum InvocationType {
    RequestResponse,
    Event,
    DryRun,
}

impl std::str::FromStr for InvocationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "RequestResponse" => Ok(InvocationType::RequestResponse),
            "Event" => Ok(InvocationType::Event),
            "DryRun" => Ok(InvocationType::DryRun),
            _ => Err(format!("Invalid invocation type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum LogType {
    None,
    Tail,
}

impl std::str::FromStr for LogType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "None" => Ok(LogType::None),
            "Tail" => Ok(LogType::Tail),
            _ => Err(format!("Invalid log type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvokeResponse {
    pub status_code: u16,
    pub payload: Option<serde_json::Value>,
    pub executed_version: Option<String>,
    pub function_error: Option<FunctionError>,
    pub log_result: Option<String>, // base64 encoded log tail
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum FunctionError {
    Handled,
    Unhandled,
}

// Runtime API types (for containers)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeInvocation {
    pub aws_request_id: Uuid,
    pub deadline_ms: i64,
    pub invoked_function_arn: String,
    pub trace_id: Option<String>,
    pub client_context: Option<String>,
    pub cognito_identity: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeResponse {
    pub aws_request_id: Uuid,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeError {
    pub aws_request_id: Uuid,
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitError {
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<Vec<String>>,
}

// Execution tracking

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub execution_id: Uuid,
    pub function_id: Uuid,
    pub function_version: String,
    pub aws_request_id: Uuid,
    pub container_id: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub billed_ms: Option<u64>,
    pub memory_used_mb: Option<u64>,
    pub error_type: Option<ErrorType>,
    pub status: ExecutionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Success,
    Error,
    Timeout,
    Throttled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub enum ErrorType {
    InitError,
    BadRequest,
    Throttled,
    Timeout,
    OOMKilled,
    Unhandled,
    Handled,
}
