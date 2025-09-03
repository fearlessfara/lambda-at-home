use serde::{Deserialize, Serialize};
use thiserror::Error;
use sqlx;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorShape {
    pub error_message: String,
    pub error_type: String,
    pub stack_trace: Option<Vec<String>>,
}

#[derive(Error, Debug)]
pub enum LambdaError {
    #[error("Function not found: {function_name}")]
    FunctionNotFound { function_name: String },
    
    #[error("Function already exists: {function_name}")]
    FunctionAlreadyExists { function_name: String },
    
    #[error("Invalid function name: {function_name}")]
    InvalidFunctionName { function_name: String },
    
    #[error("Invalid runtime: {runtime}")]
    InvalidRuntime { runtime: String },
    
    #[error("Invalid handler: {handler}")]
    InvalidHandler { handler: String },
    
    #[error("Code too large: {size} bytes (max: {max_size})")]
    CodeTooLarge { size: u64, max_size: u64 },
    
    #[error("Invalid ZIP file: {reason}")]
    InvalidZipFile { reason: String },
    
    #[error("Docker error: {message}")]
    DockerError { message: String },
    
    #[error("Container timeout after {timeout_ms}ms")]
    ContainerTimeout { timeout_ms: u64 },
    
    #[error("Container out of memory")]
    ContainerOOM,
    
    #[error("Container initialization failed: {reason}")]
    ContainerInitError { reason: String },
    
    #[error("Function execution failed: {reason}")]
    FunctionExecutionError { reason: String },
    
    #[error("Concurrency limit exceeded for function: {function_name}")]
    ConcurrencyLimitExceeded { function_name: String },
    
    #[error("Global concurrency limit exceeded")]
    GlobalConcurrencyLimitExceeded,
    
    #[error("Invalid request: {reason}")]
    InvalidRequest { reason: String },
    
    #[error("Internal server error: {reason}")]
    InternalError { reason: String },
    
    #[error("Database error: {reason}")]
    DatabaseError { reason: String },
    
    #[error("SQLx error: {0}")]
    SqlxError(#[from] sqlx::Error),
    
    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },
}

impl LambdaError {
    pub fn to_error_shape(&self) -> ErrorShape {
        ErrorShape {
            error_message: self.to_string(),
            error_type: self.error_type().to_string(),
            stack_trace: None,
        }
    }
    
    pub fn error_type(&self) -> &'static str {
        match self {
            LambdaError::FunctionNotFound { .. } => "ResourceNotFoundException",
            LambdaError::FunctionAlreadyExists { .. } => "ResourceConflictException",
            LambdaError::InvalidFunctionName { .. } => "InvalidParameterValueException",
            LambdaError::InvalidRuntime { .. } => "InvalidParameterValueException",
            LambdaError::InvalidHandler { .. } => "InvalidParameterValueException",
            LambdaError::CodeTooLarge { .. } => "InvalidParameterValueException",
            LambdaError::InvalidZipFile { .. } => "InvalidParameterValueException",
            LambdaError::DockerError { .. } => "ServiceException",
            LambdaError::ContainerTimeout { .. } => "TaskTimedOutException",
            LambdaError::ContainerOOM => "OutOfMemoryError",
            LambdaError::ContainerInitError { .. } => "InitError",
            LambdaError::FunctionExecutionError { .. } => "Unhandled",
            LambdaError::ConcurrencyLimitExceeded { .. } => "TooManyRequestsException",
            LambdaError::GlobalConcurrencyLimitExceeded => "TooManyRequestsException",
            LambdaError::InvalidRequest { .. } => "InvalidParameterValueException",
            LambdaError::InternalError { .. } => "ServiceException",
            LambdaError::DatabaseError { .. } => "ServiceException",
            LambdaError::SqlxError(_) => "ServiceException",
            LambdaError::ConfigError { .. } => "ServiceException",
        }
    }
    
    pub fn http_status(&self) -> u16 {
        match self {
            LambdaError::FunctionNotFound { .. } => 404,
            LambdaError::FunctionAlreadyExists { .. } => 409,
            LambdaError::InvalidFunctionName { .. } => 400,
            LambdaError::InvalidRuntime { .. } => 400,
            LambdaError::InvalidHandler { .. } => 400,
            LambdaError::CodeTooLarge { .. } => 400,
            LambdaError::InvalidZipFile { .. } => 400,
            LambdaError::DockerError { .. } => 500,
            LambdaError::ContainerTimeout { .. } => 200, // Lambda returns 200 with error header
            LambdaError::ContainerOOM => 200, // Lambda returns 200 with error header
            LambdaError::ContainerInitError { .. } => 200, // Lambda returns 200 with error header
            LambdaError::FunctionExecutionError { .. } => 200, // Lambda returns 200 with error header
            LambdaError::ConcurrencyLimitExceeded { .. } => 429,
            LambdaError::GlobalConcurrencyLimitExceeded => 429,
            LambdaError::InvalidRequest { .. } => 400,
            LambdaError::InternalError { .. } => 500,
            LambdaError::DatabaseError { .. } => 500,
            LambdaError::SqlxError(_) => 500,
            LambdaError::ConfigError { .. } => 500,
        }
    }
}
