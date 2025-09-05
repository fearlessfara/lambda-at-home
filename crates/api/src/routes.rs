use axum::{
    routing::{get, post, put, delete, any},
    Router,
};
use crate::{AppState, handlers::*};

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
        .route("/2015-03-31/functions/:name/invocations", post(invoke_function))
        
        // Health and metrics
        .route("/healthz", get(health_check))
        .route("/metrics", get(metrics))
        .fallback(any(api_gateway_proxy))
}

pub fn build_router(state: AppState) -> Router {
    create_router().with_state(state)
}
