use lambda_at_home::{
    Config,
    DockerManager,
    LambdaRuntimeService,
    FunctionStorage,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, error};

/// Check if Docker is running and accessible
async fn is_docker_running() -> bool {
    match tokio::process::Command::new("docker")
        .args(&["version", "--format", "{{.Server.Version}}"])
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    info!("🚀 Starting Lambda Runtime API Server");

    // Check if Docker is running
    info!("🐳 Checking Docker availability...");
    if !is_docker_running().await {
        error!("❌ Docker is not running or not accessible");
        error!("   Please ensure Docker is installed and running");
        error!("   You can start Docker with: sudo systemctl start docker (Linux) or start Docker Desktop");
        std::process::exit(1);
    }
    info!("✅ Docker is running and accessible");

    // Create configuration
    let config = Config::load("config/config.toml").unwrap_or_else(|_| {
        info!("📋 Using default configuration (config file not found)");
        Config::default()
    });
    info!("📋 Configuration loaded: {:?}", config);

    // Create components
    let _docker_manager = DockerManager::new().await?;
    let _storage = Arc::new(FunctionStorage::new(&config.storage_path)?);
    let lambda_runtime_service = Arc::new(LambdaRuntimeService::new());

    info!("✅ All components initialized successfully");

    // Start the User API server (port 8080)
    let user_app = lambda_runtime_service.create_router().with_state(lambda_runtime_service.clone());
    let user_listener = TcpListener::bind(format!("{}:{}", config.server_address, config.port)).await?;
    
    info!("🌐 User API server listening on {}:{}", config.server_address, config.port);
    
    // Start the RIC API server (port 3000)
    let ric_app = lambda_runtime_service.create_router().with_state(lambda_runtime_service);
    let ric_listener = TcpListener::bind(format!("{}:{}", config.server_address, config.ric_config.port)).await?;
    
    info!("🔗 RIC API server listening on {}:{}", config.server_address, config.ric_config.port);
    info!("📡 Ready to handle Lambda invocations");

    // Start both servers concurrently
    tokio::select! {
        result = axum::serve(user_listener, user_app) => {
            if let Err(e) = result {
                error!("User API server error: {}", e);
            }
        }
        result = axum::serve(ric_listener, ric_app) => {
            if let Err(e) = result {
                error!("RIC API server error: {}", e);
            }
        }
    }

    Ok(())
}
