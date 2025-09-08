use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use axum::response::Json;

use crate::handlers::*;
use lambda_models::{
    CreateFunctionRequest, Function, FunctionState, Version, Alias, FunctionCode,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        create_function,
        get_function,
        delete_function,
        invoke_function,
        health_check,
        metrics,
    ),
    components(
        schemas(
            Function,
            FunctionState,
            Version,
            Alias,
            CreateFunctionRequest,
            FunctionCode,
        )
    ),
    tags(
        (name = "functions", description = "Function management endpoints"),
        (name = "invocation", description = "Function invocation endpoints"),
        (name = "health", description = "Health and monitoring endpoints"),
    ),
    info(
        title = "Lambda@Home API",
        description = "AWS Lambda-compatible serverless function runtime for local development",
        version = "0.1.0",
        contact(
            name = "Lambda@Home",
            url = "https://github.com/lambda-at-home/lambda-at-home"
        )
    ),
    servers(
        (url = "http://localhost:9000", description = "Local development server")
    )
)]
pub struct ApiDoc;

pub fn create_swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui")
        .url("/openapi.json", ApiDoc::openapi())
        .config(
            utoipa_swagger_ui::Config::new(["/openapi.json"])
                .try_it_out_enabled(true)
                .display_request_duration(true)
        )
}

pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}