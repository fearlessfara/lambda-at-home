use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::{AppState, handlers::*, handlers::warm_pool_summary};

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Function management
        .route("/2015-03-31/functions", post(create_function))
        .route("/2015-03-31/functions/:name", get(get_function))
        .route("/2015-03-31/functions/:name", delete(delete_function))
        .route("/2015-03-31/functions", get(list_functions))
        
        // Function code and configuration
        .route("/2015-03-31/functions/:name/code", put(update_function_code))
        .route("/2015-03-31/functions/:name/configuration", put(update_function_configuration))
        
        // Versions
        .route("/2015-03-31/functions/:name/versions", post(publish_version))
        .route("/2015-03-31/functions/:name/versions", get(list_versions))
        
        // Aliases
        .route("/2015-03-31/functions/:name/aliases", post(create_alias))
        .route("/2015-03-31/functions/:name/aliases/:alias", get(get_alias))
        .route("/2015-03-31/functions/:name/aliases/:alias", put(update_alias))
        .route("/2015-03-31/functions/:name/aliases/:alias", delete(delete_alias))
        .route("/2015-03-31/functions/:name/aliases", get(list_aliases))
        
        // Concurrency
        .route("/2015-03-31/functions/:name/concurrency", put(put_concurrency))
        .route("/2015-03-31/functions/:name/concurrency", get(get_concurrency))
        .route("/2015-03-31/functions/:name/concurrency", delete(delete_concurrency))
        
        // Invocation
        .route(
            "/2015-03-31/functions/:name/invocations",
            post(|state, path, headers, body| async move { invoke_function(state, path, headers, body).await })
        )
        
        // Health and metrics
        .route("/healthz", get(health_check))
        .route("/metrics", get(metrics))
        // Warm pool admin
        .route("/admin/warm-pool/:name", get(warm_pool_summary))
        // API Gateway routes admin
        .route("/admin/api-gateway/routes", get(list_api_routes))
        .route("/admin/api-gateway/routes", post(create_api_route))
        .route("/admin/api-gateway/routes/:id", delete(delete_api_route))
        // Secrets admin
        .route("/admin/secrets", get(list_secrets))
        .route("/admin/secrets", post(create_secret))
        .route("/admin/secrets/:name", delete(delete_secret))
        .fallback(|state, req| async move { api_gateway_proxy(state, req).await })
}

pub fn build_router(state: AppState) -> Router {
    create_router().with_state(state)
}
