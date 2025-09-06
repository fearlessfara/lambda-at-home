pub mod handlers;
pub mod routes;
pub mod state;

pub use routes::build_router;
pub use state::RtState;

use axum::Router;
use lambda_control::{pending::Pending, queues::Queues, ControlPlane};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

pub async fn start_server(
    bind: String,
    port: u16,
    control_plane: Arc<ControlPlane>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // State shares the control plane so runtime API uses global queues/pending
    let app_state = RtState {
        control: Some(control_plane),
        // Keep local queues/pending for potential test-only fallbacks
        queues: Queues::new(),
        pending: Pending::new(),
    };

    let app = Router::new().merge(build_router(app_state.clone())).layer(
        ServiceBuilder::new()
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::permissive()),
    );

    let listener = tokio::net::TcpListener::bind(format!("{bind}:{port}")).await?;
    info!("Runtime API server listening on {}:{}", bind, port);

    axum::serve(listener, app).await?;
    Ok(())
}
