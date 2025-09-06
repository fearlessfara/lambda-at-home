use crate::AppState;
use axum::{
    body::Body,
    body::Bytes,
    extract::{Path, Query, State},
    http::Request,
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::IntoResponse,
    response::Json,
};
use lambda_models::{
    ApiRoute, ConcurrencyConfig, CreateAliasRequest, CreateApiRouteRequest, CreateFunctionRequest,
    CreateSecretRequest, ErrorShape, FunctionError, InvokeRequest, ListAliasesResponse,
    ListApiRoutesResponse, ListFunctionsResponse, ListSecretsResponse, ListVersionsResponse,
    PublishVersionRequest, SecretListItem, UpdateAliasRequest, UpdateFunctionCodeRequest,
    UpdateFunctionConfigurationRequest,
};
use std::collections::HashMap;
use tracing::{error, info, instrument};

#[instrument(skip(state))]
pub async fn create_function(
    State(state): State<AppState>,
    Json(payload): Json<CreateFunctionRequest>,
) -> Result<Json<lambda_models::Function>, (StatusCode, Json<ErrorShape>)> {
    info!("Creating function: {}", payload.function_name);

    match state.control.create_function(payload).await {
        Ok(function) => {
            state
                .metrics
                .record_function_created(&function.function_name)
                .await;
            Ok(Json(function))
        }
        Err(e) => {
            error!("Failed to create function: {}", e);
            let error_shape = e.to_error_shape();
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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

    match state
        .control
        .update_function_configuration(&name, payload)
        .await
    {
        Ok(function) => Ok(Json(function)),
        Err(e) => {
            error!(
                "Failed to update function configuration for {}: {}",
                name, e
            );
            let error_shape = e.to_error_shape();
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
        }
    }
}

// -------- Secrets admin --------
#[instrument(skip(state))]
pub async fn list_secrets(
    State(state): State<AppState>,
) -> Result<Json<ListSecretsResponse>, (StatusCode, Json<ErrorShape>)> {
    match state.control.list_secrets().await {
        Ok(items) => {
            let secrets = items
                .into_iter()
                .map(|(name, created_at)| SecretListItem { name, created_at })
                .collect();
            Ok(Json(ListSecretsResponse { secrets }))
        }
        Err(e) => Err((
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(e.to_error_shape()),
        )),
    }
}

#[instrument(skip(state))]
pub async fn create_secret(
    State(state): State<AppState>,
    Json(payload): Json<CreateSecretRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorShape>)> {
    match state
        .control
        .create_secret(&payload.name, &payload.value)
        .await
    {
        Ok(()) => Ok(StatusCode::CREATED),
        Err(e) => Err((
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(e.to_error_shape()),
        )),
    }
}

#[instrument(skip(state))]
pub async fn delete_secret(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorShape>)> {
    match state.control.delete_secret(&name).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(e.to_error_shape()),
        )),
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
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
                Some(serde_json::Value::String(
                    String::from_utf8_lossy(&body).to_string(),
                ))
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
            Err((
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                Json(error_shape),
            ))
        }
    }
}

#[instrument(skip(_state))]
pub async fn health_check(State(_state): State<AppState>) -> Result<&'static str, StatusCode> {
    Ok("OK")
}

#[instrument(skip(state))]
pub async fn metrics(State(state): State<AppState>) -> Result<String, StatusCode> {
    match state.metrics.get_prometheus_metrics().await {
        Ok(metrics) => Ok(metrics),
        Err(e) => {
            error!("Failed to get metrics: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[instrument(skip(state))]
pub async fn warm_pool_summary(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<lambda_control::warm_pool::WarmPoolSummary>, (StatusCode, Json<ErrorShape>)> {
    let summary = state.control.warm_pool().summary_for_function(&name).await;
    Ok(Json(summary))
}

// -------- API Gateway routes admin --------
#[instrument(skip(state))]
pub async fn list_api_routes(
    State(state): State<AppState>,
) -> Result<Json<ListApiRoutesResponse>, (StatusCode, Json<ErrorShape>)> {
    match state.control.list_api_routes().await {
        Ok(resp) => Ok(Json(resp)),
        Err(e) => Err((
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(e.to_error_shape()),
        )),
    }
}

#[instrument(skip(state))]
pub async fn create_api_route(
    State(state): State<AppState>,
    Json(payload): Json<CreateApiRouteRequest>,
) -> Result<Json<ApiRoute>, (StatusCode, Json<ErrorShape>)> {
    match state.control.create_api_route(payload).await {
        Ok(route) => Ok(Json(route)),
        Err(e) => Err((
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(e.to_error_shape()),
        )),
    }
}

#[instrument(skip(state))]
pub async fn delete_api_route(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ErrorShape>)> {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorShape {
                    error_message: "Invalid route id".into(),
                    error_type: "BadRequest".into(),
                    stack_trace: None,
                }),
            ))
        }
    };
    match state.control.delete_api_route(uuid).await {
        Ok(()) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((
            StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(e.to_error_shape()),
        )),
    }
}

// API Gateway-style proxy: path name equals function name
// Captures any unmatched path and invokes a function named by the first segment.
#[instrument(skip(state, req))]
pub async fn api_gateway_proxy(
    State(state): State<AppState>,
    req: Request<Body>,
) -> impl IntoResponse {
    let uri = req.uri().clone();
    let path = uri.path().to_string();
    let mut segs = path.trim_start_matches('/').split('/');
    // First try to resolve via configured API routes (longest prefix, optional method)
    let method_str = req.method().to_string();
    let resolved = match state.control.resolve_api_route(&method_str, &path).await {
        Ok(r) => r,
        Err(_) => None,
    };
    let mut from_mapping = false;
    let func_name = if let Some(f) = resolved {
        from_mapping = true;
        f
    } else {
        match segs.next() {
            Some(s) if !s.is_empty() => s.to_string(),
            _ => return (StatusCode::NOT_FOUND, Body::from("Not Found")).into_response(),
        }
    };

    // If no mapping was found and we're about to treat the first path segment as a function name,
    // verify that the function actually exists. If not, return a clean 404 instead of invoking.
    if !from_mapping {
        if state.control.get_function(&func_name).await.is_err() {
            return (StatusCode::NOT_FOUND, Body::from("Not Found")).into_response();
        }
    }

    // Build API Gateway proxy-like event
    let method = req.method().to_string();
    let query_map = uri
        .query()
        .map(|q| {
            form_urlencoded::parse(q.as_bytes())
                .into_owned()
                .collect::<std::collections::HashMap<String, String>>()
        })
        .unwrap_or_default();
    let headers_map: std::collections::HashMap<String, String> = req
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|s| (k.to_string(), s.to_string())))
        .collect();
    let whole_body = axum::body::to_bytes(req.into_body(), 1024 * 1024)
        .await
        .unwrap_or_else(|_| Bytes::new());
    let body_str = String::from_utf8_lossy(&whole_body).to_string();

    let event = serde_json::json!({
        "resource": path,
        "path": path,
        "httpMethod": method,
        "headers": headers_map,
        "queryStringParameters": query_map,
        "pathParameters": serde_json::Value::Null,
        "stageVariables": serde_json::Value::Null,
        "requestContext": { "path": path },
        "body": if body_str.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(body_str.clone()) },
        "isBase64Encoded": false
    });

    let request = lambda_models::InvokeRequest {
        function_name: func_name.clone(),
        invocation_type: lambda_models::InvocationType::RequestResponse,
        log_type: None,
        client_context: None,
        payload: Some(event),
        qualifier: None,
    };

    match state.control.invoke_function(request).await {
        Ok(resp) => {
            // If the function returned an API Gateway proxy result (statusCode/body/headers), map it.
            if let Some(payload) = &resp.payload {
                if let Some(status) = payload.get("statusCode").and_then(|v| v.as_u64()) {
                    let mut headers = HeaderMap::new();
                    if let Some(hs) = payload.get("headers").and_then(|v| v.as_object()) {
                        for (k, v) in hs.iter() {
                            if let Some(s) = v.as_str() {
                                if let Ok(name) = HeaderName::from_bytes(k.as_bytes()) {
                                    if let Ok(val) = HeaderValue::from_str(s) {
                                        headers.insert(name, val);
                                    }
                                }
                            }
                        }
                    }
                    let body = payload
                        .get("body")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let body_bytes = if body.is_string() {
                        Body::from(body.as_str().unwrap().to_owned())
                    } else {
                        Body::from(body.to_string())
                    };
                    return (
                        StatusCode::from_u16(status as u16).unwrap_or(StatusCode::OK),
                        headers,
                        body_bytes,
                    )
                        .into_response();
                }
            }
            // Default mapping heuristics:
            // - If payload is an object with a 'body' field, treat like APIGW result (status default 200)
            // - If payload is a bare string, return as text body
            // - Else return JSON payload
            let status = StatusCode::from_u16(resp.status_code).unwrap_or(StatusCode::OK);
            let mut headers = HeaderMap::new();
            for (k, v) in resp.headers {
                if let Ok(name) = HeaderName::from_bytes(k.as_bytes()) {
                    if let Ok(val) = HeaderValue::from_str(&v) {
                        headers.insert(name, val);
                    }
                }
            }
            if let Some(payload) = &resp.payload {
                if let Some(obj) = payload.as_object() {
                    if let Some(body) = obj.get("body") {
                        let status = obj
                            .get("statusCode")
                            .and_then(|v| v.as_u64())
                            .map(|s| StatusCode::from_u16(s as u16).unwrap_or(StatusCode::OK))
                            .unwrap_or(status);
                        if let Some(hs) = obj.get("headers").and_then(|v| v.as_object()) {
                            for (k, v) in hs.iter() {
                                if let Some(s) = v.as_str() {
                                    if let Ok(name) = HeaderName::from_bytes(k.as_bytes()) {
                                        if let Ok(val) = HeaderValue::from_str(s) {
                                            headers.insert(name, val);
                                        }
                                    }
                                }
                            }
                        }
                        let body_bytes = if body.is_string() {
                            Body::from(body.as_str().unwrap().to_owned())
                        } else {
                            Body::from(body.to_string())
                        };
                        return (status, headers, body_bytes).into_response();
                    }
                }
                if payload.is_string() {
                    return (
                        status,
                        headers,
                        Body::from(payload.as_str().unwrap().to_owned()),
                    )
                        .into_response();
                }
            }
            (
                status,
                headers,
                Json(resp.payload.unwrap_or(serde_json::Value::Null)),
            )
                .into_response()
        }
        Err(e) => {
            let status =
                StatusCode::from_u16(e.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, Json(e.to_error_shape())).into_response()
        }
    }
}
