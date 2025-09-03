use lambda_at_home::{
    core::config::Config,
    docker::container_lifecycle::ContainerLifecycleManager,
    docker::docker::docker::DockerManager,
    core::models::{Function, FunctionStatus},
    core::storage::FunctionStorage,
};
use std::sync::Arc;
use tracing::{info, warn};
use uuid::Uuid;

#[tokio::test]
async fn test_lambda_debug_container_creation() {
    // Initialize logging
    let _ = tracing_subscriber::fmt::try_init();

    info!("🚀 Starting Lambda Debug Container Creation Test");

    // Create test configuration
    let config = Config::default();

    // Create components
    let docker_manager = DockerManager::new().await.expect("Failed to create Docker manager");
    let storage = Arc::new(FunctionStorage::new(&config.storage_path).expect("Failed to create storage"));
    let lifecycle_manager = Arc::new(ContainerLifecycleManager::new(
        docker_manager.clone(),
        config.clone(),
        storage.clone(),
    ));

    // Create a test function
    let function_id = Uuid::new_v4();
    let function_name = format!("debug-test-{}", function_id);
    
    info!("📝 Creating debug test function: {}", function_name);

    // Create function metadata
    let function_metadata = Function {
        id: function_id,
        name: function_name.clone(),
        description: Some("Debug test function".to_string()),
        runtime: "nodejs".to_string(),
        handler: "index.handler".to_string(),
        status: FunctionStatus::Ready,
        docker_image: Some(format!("lambda-function-{}", function_id)),
        memory_size: Some(128),
        cpu_limit: Some(0.5),
        timeout: Some(30),
        environment: None,
        created_at: chrono::Utc::now(),
    };

    // Store function metadata
    storage.save_function(&function_metadata).await.expect("Failed to store function");

    // Try to get or create a container
    info!("🧪 Testing container creation");
    
    match lifecycle_manager.get_or_create_container(&function_id).await {
        Ok(container_info) => {
            info!("✅ Container created successfully: {:?}", container_info);
            
            // Check if container is actually running
            match docker_manager.is_container_running(&container_info.id).await {
                Ok(is_running) => {
                    if is_running {
                        info!("✅ Container is running");
                    } else {
                        warn!("⚠️ Container is not running");
                    }
                }
                Err(e) => {
                    warn!("❌ Failed to check container status: {}", e);
                }
            }
        }
        Err(e) => {
            warn!("❌ Failed to create container: {}", e);
            
            // Let's try to see what containers exist
            match docker_manager.list_containers().await {
                Ok(containers) => {
                    info!("📊 Available containers: {}", containers.len());
                    for container in containers.iter().take(5) {
                        info!("   Container: {:?}", container);
                    }
                }
                Err(e) => {
                    warn!("❌ Failed to list containers: {}", e);
                }
            }
        }
    }

    info!("🎉 Lambda Debug Container Creation Test completed!");
}
