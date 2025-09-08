use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Function {
    pub function_id: Uuid,
    pub function_name: String,
    pub runtime: String,
    pub role: Option<String>,
    pub handler: String,
    pub code_sha256: String,
    pub description: Option<String>,
    pub timeout: u64,
    pub memory_size: u64,
    pub environment: HashMap<String, String>,
    pub last_modified: DateTime<Utc>,
    pub code_size: u64,
    pub version: String,
    pub state: FunctionState,
    pub state_reason: Option<String>,
    pub state_reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
#[serde(deny_unknown_fields)]
pub enum FunctionState {
    Pending,
    Active,
    Inactive,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub version_id: Uuid,
    pub function_id: Uuid,
    pub version: String,
    pub description: Option<String>,
    pub code_sha256: String,
    pub last_modified: DateTime<Utc>,
    pub code_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Alias {
    pub alias_id: Uuid,
    pub function_id: Uuid,
    pub name: String,
    pub function_version: String,
    pub description: Option<String>,
    pub routing_config: Option<RoutingConfig>,
    pub revision_id: String,
    pub last_modified: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoutingConfig {
    pub additional_version_weights: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConcurrencyConfig {
    pub reserved_concurrent_executions: Option<u32>,
}

// Request/Response types for API

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateFunctionRequest {
    pub function_name: String,
    pub runtime: String,
    pub role: Option<String>,
    pub handler: String,
    pub code: FunctionCode,
    pub description: Option<String>,
    pub timeout: Option<u64>,
    pub memory_size: Option<u64>,
    pub environment: Option<HashMap<String, String>>,
    pub publish: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FunctionCode {
    pub zip_file: Option<String>, // base64 encoded
    pub s3_bucket: Option<String>,
    pub s3_key: Option<String>,
    pub s3_object_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateFunctionCodeRequest {
    pub zip_file: Option<String>, // base64 encoded
    pub s3_bucket: Option<String>,
    pub s3_key: Option<String>,
    pub s3_object_version: Option<String>,
    pub publish: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateFunctionConfigurationRequest {
    pub role: Option<String>,
    pub handler: Option<String>,
    pub description: Option<String>,
    pub timeout: Option<u64>,
    pub memory_size: Option<u64>,
    pub environment: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishVersionRequest {
    pub description: Option<String>,
    pub revision_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateAliasRequest {
    pub name: String,
    pub function_version: String,
    pub description: Option<String>,
    pub routing_config: Option<RoutingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateAliasRequest {
    pub function_version: Option<String>,
    pub description: Option<String>,
    pub routing_config: Option<RoutingConfig>,
    pub revision_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListFunctionsResponse {
    pub functions: Vec<Function>,
    pub next_marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListVersionsResponse {
    pub versions: Vec<Version>,
    pub next_marker: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListAliasesResponse {
    pub aliases: Vec<Alias>,
    pub next_marker: Option<String>,
}
