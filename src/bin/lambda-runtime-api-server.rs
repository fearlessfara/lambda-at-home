use lambda_docker_executor::{
    config::Config,
    docker::DockerManager,
    lambda_runtime_api::LambdaRuntimeService,
    storage::FunctionStorage,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Lambda Runtime API Server");

    // Create configuration
    let config = Config::default();
    info!("📋 Configuration loaded: {:?}", config);

    // Create components
    let docker_manager = DockerManager::new().await?;
    let storage = Arc::new(FunctionStorage::new(&config.storage_path)?);
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::new());

    info!("✅ All components initialized successfully");

    // Start the Lambda Runtime API server
    let app = lambda_runtime_service.create_router().with_state(lambda_runtime_service);
    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    
    info!("🌐 Lambda Runtime API server listening on 0.0.0.0:8080");
    info!("📡 Ready to handle Lambda invocations");

    // Start the server
    axum::serve(listener, app).await?;

    Ok(())
}
