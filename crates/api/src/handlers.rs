use axum::{
    extract::{Path, State, Query},
    http::{HeaderMap, StatusCode, HeaderValue, HeaderName},
    response::Json,
    body::Bytes,
};
use std::collections::HashMap;
use lambda_models::{
    CreateFunctionRequest, UpdateFunctionCodeRequest, UpdateFunctionConfigurationRequest,
    PublishVersionRequest, CreateAliasRequest, UpdateAliasRequest, ConcurrencyConfig,
    ListFunctionsResponse, ListVersionsResponse, ListAliasesResponse, InvokeRequest,
    FunctionError, ErrorShape,
};
use crate::AppState;
use tracing::{info, error, instrument};

#[instrument(skip(state))]
pub async fn create_function(
    State(state): State<AppState>,
    Json(payload): Json<CreateFunctionRequest>,
) -> Result<Json<lambda_models::Function>, (StatusCode, Json<ErrorShape>)> {
    info!("Creating function: {}", payload.function_name);
    
    match state.control.create_function(payload).await {
        Ok(function) => {
            state.metrics.record_function_created(&function.function_name).await;
            Ok(Json(function))
        }
        Err(e) => {
            error!("Failed to create function: {}", e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn get_function(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<lambda_models::Function>, (StatusCode, Json<ErrorShape>)> {
    info!("Getting function: {}", name);
    
    match state.control.get_function(&name).await {
        Ok(function) => Ok(Json(function)),
        Err(e) => {
            error!("Failed to get function {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn delete_function(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorShape>)> {
    info!("Deleting function: {}", name);
    
    match state.control.delete_function(&name).await {
        Ok(_) => {
            state.metrics.record_function_deleted(&name).await;
            Ok(StatusCode::NO_CONTENT)
        }
        Err(e) => {
            error!("Failed to delete function {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn list_functions(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListFunctionsResponse>, (StatusCode, Json<ErrorShape>)> {
    let marker = params.get("Marker");
    let max_items = params.get("MaxItems").and_then(|s| s.parse::<u32>().ok());
    
    match state.control.list_functions(marker, max_items).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("Failed to list functions: {}", e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn update_function_code(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<UpdateFunctionCodeRequest>,
) -> Result<Json<lambda_models::Function>, (StatusCode, Json<ErrorShape>)> {
    info!("Updating function code: {}", name);
    
    match state.control.update_function_code(&name, payload).await {
        Ok(function) => Ok(Json(function)),
        Err(e) => {
            error!("Failed to update function code for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn update_function_configuration(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<UpdateFunctionConfigurationRequest>,
) -> Result<Json<lambda_models::Function>, (StatusCode, Json<ErrorShape>)> {
    info!("Updating function configuration: {}", name);
    
    match state.control.update_function_configuration(&name, payload).await {
        Ok(function) => Ok(Json(function)),
        Err(e) => {
            error!("Failed to update function configuration for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn publish_version(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<PublishVersionRequest>,
) -> Result<Json<lambda_models::Version>, (StatusCode, Json<ErrorShape>)> {
    info!("Publishing version for function: {}", name);
    
    match state.control.publish_version(&name, payload).await {
        Ok(version) => Ok(Json(version)),
        Err(e) => {
            error!("Failed to publish version for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn list_versions(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListVersionsResponse>, (StatusCode, Json<ErrorShape>)> {
    let marker = params.get("Marker");
    let max_items = params.get("MaxItems").and_then(|s| s.parse::<u32>().ok());
    
    match state.control.list_versions(&name, marker, max_items).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("Failed to list versions for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn create_alias(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<CreateAliasRequest>,
) -> Result<Json<lambda_models::Alias>, (StatusCode, Json<ErrorShape>)> {
    info!("Creating alias {} for function: {}", payload.name, name);
    
    match state.control.create_alias(&name, payload).await {
        Ok(alias) => Ok(Json(alias)),
        Err(e) => {
            error!("Failed to create alias for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn get_alias(
    State(state): State<AppState>,
    Path((name, alias)): Path<(String, String)>,
) -> Result<Json<lambda_models::Alias>, (StatusCode, Json<ErrorShape>)> {
    info!("Getting alias {} for function: {}", alias, name);
    
    match state.control.get_alias(&name, &alias).await {
        Ok(alias) => Ok(Json(alias)),
        Err(e) => {
            error!("Failed to get alias {} for {}: {}", alias, name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn update_alias(
    State(state): State<AppState>,
    Path((name, alias)): Path<(String, String)>,
    Json(payload): Json<UpdateAliasRequest>,
) -> Result<Json<lambda_models::Alias>, (StatusCode, Json<ErrorShape>)> {
    info!("Updating alias {} for function: {}", alias, name);
    
    match state.control.update_alias(&name, &alias, payload).await {
        Ok(alias) => Ok(Json(alias)),
        Err(e) => {
            error!("Failed to update alias {} for {}: {}", alias, name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn delete_alias(
    State(state): State<AppState>,
    Path((name, alias)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrorShape>)> {
    info!("Deleting alias {} for function: {}", alias, name);
    
    match state.control.delete_alias(&name, &alias).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            error!("Failed to delete alias {} for {}: {}", alias, name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn list_aliases(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListAliasesResponse>, (StatusCode, Json<ErrorShape>)> {
    let marker = params.get("Marker");
    let max_items = params.get("MaxItems").and_then(|s| s.parse::<u32>().ok());
    
    match state.control.list_aliases(&name, marker, max_items).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => {
            error!("Failed to list aliases for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn put_concurrency(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<ConcurrencyConfig>,
) -> Result<StatusCode, (StatusCode, Json<ErrorShape>)> {
    info!("Setting concurrency for function: {}", name);
    
    match state.control.put_concurrency(&name, payload).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            error!("Failed to set concurrency for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn get_concurrency(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ConcurrencyConfig>, (StatusCode, Json<ErrorShape>)> {
    info!("Getting concurrency for function: {}", name);
    
    match state.control.get_concurrency(&name).await {
        Ok(config) => Ok(Json(config)),
        Err(e) => {
            error!("Failed to get concurrency for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state))]
pub async fn delete_concurrency(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorShape>)> {
    info!("Deleting concurrency for function: {}", name);
    
    match state.control.delete_concurrency(&name).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            error!("Failed to delete concurrency for {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(state, headers, body))]
pub async fn invoke_function(
    State(state): State<AppState>,
    Path(name): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, HeaderMap, Json<serde_json::Value>), (StatusCode, Json<ErrorShape>)> {
    info!("Invoking function: {}", name);
    
    // Parse invocation type from headers
    let invocation_type = headers
        .get("X-Amz-Invocation-Type")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(lambda_models::InvocationType::RequestResponse);
    
    let log_type = headers
        .get("X-Amz-Log-Type")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok());
    
    // Parse payload
    let payload = if body.is_empty() {
        None
    } else {
        match serde_json::from_slice(&body) {
            Ok(p) => Some(p),
            Err(_) => {
                // If not valid JSON, treat as string
                Some(serde_json::Value::String(String::from_utf8_lossy(&body).to_string()))
            }
        }
    };
    
    let request = InvokeRequest {
        function_name: name.clone(),
        invocation_type,
        log_type,
        client_context: None,
        payload,
        qualifier: None,
    };
    
    match state.control.invoke_function(request).await {
        Ok(response) => {
            let mut response_headers = HeaderMap::new();
            
            if let Some(executed_version) = &response.executed_version {
                if let Ok(header_value) = HeaderValue::from_str(executed_version) {
                    response_headers.insert("X-Amz-Executed-Version", header_value);
                }
            }
            
            if let Some(log_result) = &response.log_result {
                if let Ok(header_value) = HeaderValue::from_str(log_result) {
                    response_headers.insert("X-Amz-Log-Result", header_value);
                }
            }
            
            if let Some(function_error) = &response.function_error {
                let error_str = match function_error {
                    FunctionError::Handled => "Handled",
                    FunctionError::Unhandled => "Unhandled",
                };
                if let Ok(header_value) = HeaderValue::from_str(error_str) {
                    response_headers.insert("X-Amz-Function-Error", header_value);
                }
            }
            
            // Add custom headers
            for (key, value) in response.headers {
                if let Ok(header_name) = HeaderName::from_bytes(key.as_bytes()) {
                    if let Ok(header_value) = HeaderValue::from_str(&value) {
                        response_headers.insert(header_name, header_value);
                    }
                }
            }
            
            let status_code = StatusCode::from_u16(response.status_code).unwrap_or(StatusCode::OK);
            let payload = response.payload.unwrap_or(serde_json::Value::Null);
            
            Ok((status_code, response_headers, Json(payload)))
        }
        Err(e) => {
            error!("Failed to invoke function {}: {}", name, e);
            let error_shape = e.to_error_shape();
            Err((StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR), Json(error_shape)))
        }
    }
}

#[instrument(skip(_state))]
pub async fn health_check(
    State(_state): State<AppState>,
) -> Result<&'static str, StatusCode> {
    Ok("OK")
}

#[instrument(skip(state))]
pub async fn metrics(
    State(state): State<AppState>,
) -> Result<String, StatusCode> {
    match state.metrics.get_prometheus_metrics().await {
        Ok(metrics) => Ok(metrics),
        Err(e) => {
            error!("Failed to get metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
