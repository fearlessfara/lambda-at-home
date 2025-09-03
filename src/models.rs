use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub runtime: String,
    pub handler: String,
    pub status: FunctionStatus,
    pub docker_image: Option<String>,
    pub memory_size: Option<u32>,
    pub cpu_limit: Option<f64>,
    pub timeout: Option<u32>,
    pub environment: Option<HashMap<String, String>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Function {
    pub fn new(id: Uuid, request: &DeployFunctionRequest, docker_image: Option<String>) -> Self {
        Self {
            id,
            name: request.name.clone(),
            description: request.description.clone(),
            runtime: request.runtime.clone(),
            handler: request.handler.clone(),
            status: FunctionStatus::Pending,
            docker_image,
            memory_size: request.memory_size,
            cpu_limit: request.cpu_limit,
            timeout: request.timeout,
            environment: request.environment.clone(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.status == FunctionStatus::Ready
    }

    pub fn update_status(&mut self, status: FunctionStatus) {
        self.status = status;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployFunctionRequest {
    pub name: String,
    pub description: Option<String>,
    pub runtime: String,
    pub handler: String,
    pub code: FunctionCode,
    pub memory_size: Option<u32>,
    pub cpu_limit: Option<f64>,
    pub timeout: Option<u32>,
    pub environment: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCode {
    pub zip_file: String,
    pub dockerfile: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FunctionStatus {
    Pending,
    Building,
    Ready,
    Failed,
}

// Lambda-style invocation models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeFunctionRequest {
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvokeFunctionResponse {
    pub status_code: i32,
    pub function_error: Option<String>,
    pub logs: Vec<String>,
    pub payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RICInvokeRequest {
    pub payload: Value,
    pub timeout: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RICInvokeResponse {
    pub status_code: i32,
    pub body: RICResponseBody,
    pub logs: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone)]
pub struct RICResponseBody {
    pub status_code: i32,
    pub body: serde_json::Value, // This can be either a string or a parsed JSON object
}

// Container statistics
#[derive(Debug, Clone, Default)]
pub struct ContainerStats {
    pub total_containers: usize,
    pub active_containers: usize,
    pub idle_containers: usize,
    pub total_invocations: u64,
    pub containers: Vec<ContainerInfo>,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub status: String,
    pub invocation_count: u64,
    pub last_used_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}