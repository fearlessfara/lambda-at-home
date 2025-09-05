pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

pub use handlers::*;
pub use middleware::*;
pub use routes::*;
pub use state::*;

use axum::{routing::get, Router};
use lambda_control::ControlPlane;
use lambda_metrics::MetricsService;
use lambda_invoker::Invoker;
use lambda_packaging::PackagingService;
use lambda_models::Config;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use rust_embed::RustEmbed;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use mime_guess;

#[derive(RustEmbed)]
#[folder = "../../console/dist"]
struct Assets;

fn embedded_file_response(path: &str) -> impl IntoResponse {
    let key = if path.is_empty() { "index.html" } else { path };
    let bytes = Assets::get(key).or_else(|| Assets::get("index.html"));
    if let Some(content) = bytes {
        let body: axum::body::Body = axum::body::Body::from(content.data.into_owned());
        let mime = mime_guess::from_path(key).first_or_octet_stream();
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("application/octet-stream")),
        );
        (StatusCode::OK, headers, body).into_response()
    } else {
        (StatusCode::NOT_FOUND, HeaderMap::new(), axum::body::Body::empty()).into_response()
    }
}

pub async fn start_server(
    bind: String,
    port: u16,
    control_plane: Arc<ControlPlane>,
    metrics: Arc<MetricsService>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create a minimal config for the packaging service
    let config = Config::default();
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
    let app = Router::new()
        .nest("/api", api)
        // Serve embedded SPA at root with fallback
        .route("/", get(|| async { embedded_file_response("") }))
        .route("/*path", get(|axum::extract::Path(p): axum::extract::Path<String>| async move { embedded_file_response(&p) }))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
        );

    let listener = tokio::net::TcpListener::bind(format!("{}:{}", bind, port)).await?;
    info!("User API server listening on {}:{}", bind, port);
    
    axum::serve(listener, app).await?;
    Ok(())
}
