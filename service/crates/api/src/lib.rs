pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

pub use handlers::*;
pub use middleware::*;
pub use routes::*;
pub use state::*;

use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use lambda_control::ControlPlane;
use lambda_invoker::Invoker;
use lambda_metrics::MetricsService;
use lambda_models::Config;
use lambda_packaging::PackagingService;
use rust_embed::RustEmbed;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

#[derive(RustEmbed)]
#[folder = "../../../console/dist"]
struct Assets;

fn embedded_file_response(path: &str) -> impl IntoResponse {
    let key = if path.is_empty() { "index.html" } else { path };

    // For SPA routes, we need to serve index.html for any route that doesn't correspond
    // to an actual static file. Check if the requested path has a file extension.
    let is_static_file = key.contains('.') && !key.ends_with(".html");

    let bytes = if is_static_file {
        // For static files (CSS, JS, images, etc.), try to find the actual file
        Assets::get(key).or_else(|| Assets::get("index.html"))
    } else {
        // For SPA routes (no extension or .html), always serve index.html
        Assets::get("index.html")
    };

    if let Some(content) = bytes {
        let body: axum::body::Body = axum::body::Body::from(content.data.into_owned());
        let mime = if is_static_file {
            mime_guess::from_path(key).first_or_octet_stream()
        } else {
            mime_guess::from_path("index.html").first_or_octet_stream()
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("text/html")),
        );
        (StatusCode::OK, headers, body).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            HeaderMap::new(),
            axum::body::Body::empty(),
        )
            .into_response()
    }
}

pub async fn start_server(
    bind: String,
    port: u16,
    control_plane: Arc<ControlPlane>,
    metrics: Arc<MetricsService>,
    config: Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Use the provided config
    let invoker = Arc::new(Invoker::new(config.clone()).await?);
    let packaging = Arc::new(PackagingService::new(config.clone()));

    let app_state = AppState {
        config,
        control: control_plane,
        invoker,
        packaging,
        metrics,
    };

    // Build API (AWS-compatible) under /api
    let api = build_router(app_state.clone());

    // Calculate body size limit in bytes
    let body_size_limit = (app_state.config.server.max_request_body_size_mb * 1024 * 1024) as usize;

    let app = Router::new()
        // AWS Lambda-compatible endpoints at root level
        .merge(api)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(DefaultBodyLimit::max(body_size_limit)),
        );

    let listener = tokio::net::TcpListener::bind(format!("{bind}:{port}")).await?;
    info!("User API server listening on {}:{}", bind, port);

    axum::serve(listener, app).await?;
    Ok(())
}

/// Start the console server that serves only the frontend
pub async fn start_console_server(
    bind: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use axum::{
        routing::get,
        Router,
    };
    use tower::ServiceBuilder;
    use tower_http::{
        cors::CorsLayer,
        trace::TraceLayer,
    };
    use tracing::info;

    let app = Router::new()
        // Serve static files for the console
        .route("/*path", get(|axum::extract::Path(p): axum::extract::Path<String>| async move {
            embedded_file_response(&p)
        }))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    let listener = tokio::net::TcpListener::bind(format!("{bind}:{port}")).await?;
    info!("Console server listening on {}:{}", bind, port);

    axum::serve(listener, app).await?;
    Ok(())
}
