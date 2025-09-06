use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use lambda_api::routes::create_router;
use lambda_api::state::AppState;
use lambda_models::Config;
use std::sync::Arc;
use tower::util::ServiceExt;

async fn create_test_app_state() -> AppState {
    let config = Config::default();
    AppState {
        config: config.clone(),
        control: Arc::new(
            lambda_control::ControlPlane::new(
                sqlx::SqlitePool::connect(":memory:").await.unwrap(),
                Arc::new(lambda_invoker::Invoker::new(config.clone()).await.unwrap()),
                config.clone(),
            )
            .await
            .unwrap(),
        ),
        invoker: Arc::new(lambda_invoker::Invoker::new(config.clone()).await.unwrap()),
        packaging: Arc::new(lambda_packaging::PackagingService::new(config.clone())),
        metrics: Arc::new(lambda_metrics::MetricsService::new().unwrap()),
    }
}

#[tokio::test]
async fn health_endpoint_works() {
    let state = create_test_app_state().await;
    let app = create_router().with_state(state);

    let res = app
        .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn metrics_endpoint_works() {
    let state = create_test_app_state().await;
    let app = create_router().with_state(state);

    let res = app
        .oneshot(Request::get("/metrics").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}
