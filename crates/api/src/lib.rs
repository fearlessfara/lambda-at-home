pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod state;

pub use handlers::*;
pub use middleware::*;
pub use routes::*;
pub use state::*;

use axum::Router;
use lambda_control::ControlPlane;
use lambda_metrics::MetricsService;
use lambda_invoker::Invoker;
use lambda_packaging::PackagingService;
use lambda_models::Config;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

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

    let app = Router::new()
        .merge(build_router(app_state.clone()))
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

