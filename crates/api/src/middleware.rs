use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};

pub fn create_middleware_stack() -> impl tower::Layer<axum::Router> {
    ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}

// Logging is handled by TraceLayer
