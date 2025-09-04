use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use lambda_models::LambdaError;
use tracing::{info, error, instrument};
// No need for FnKey import - using function names directly

#[derive(Debug, Clone)]
pub struct WarmContainer {
    pub container_id: String,
    pub function_id: Uuid,
    pub image_ref: String,
    pub created_at: Instant,
    pub last_used: Instant,
    pub is_available: bool,
}

#[derive(Clone)]
pub struct WarmPool {
    // Key by function name for proper isolation
    containers: Arc<Mutex<HashMap<String, Vec<WarmContainer>>>>,
}

impl WarmPool {
    pub fn new() -> Self {
        Self {
            containers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_warm_container(&self, function_name: &str) -> Option<WarmContainer> {
        let mut containers = self.containers.lock().await;
        if let Some(container_list) = containers.get_mut(function_name) {
            // Find first available container for this function
            for container in container_list.iter_mut() {
                if container.is_available {
                    container.is_available = false;
                    container.last_used = Instant::now();
                    
                    info!("Reusing warm container: {} for function: {}", container.container_id, function_name);
                    return Some(container.clone());
                }
            }
        }
        
        None
    }

    #[instrument(skip(self))]
    pub async fn add_warm_container(&self, function_name: &str, container: WarmContainer) {
        let container_id = container.container_id.clone();
        
        let mut containers = self.containers.lock().await;
        containers
            .entry(function_name.to_string())
            .or_insert_with(Vec::new)
            .push(container);
        
        info!("Added warm container: {} for function: {}", 
              container_id, function_name);
    }

    #[instrument(skip(self))]
    pub async fn return_container(&self, function_name: &str, container_id: &str) -> Result<(), LambdaError> {
        let mut containers = self.containers.lock().await;
        if let Some(container_list) = containers.get_mut(function_name) {
            for container in container_list.iter_mut() {
                if container.container_id == container_id {
                    container.is_available = true;
                    container.last_used = Instant::now();
                    info!("Returned container to warm pool: {}", container_id);
                    return Ok(());
                }
            }
        }
        
        error!("Container not found in warm pool: {}", container_id);
        Err(LambdaError::InvalidRequest { reason: "Container not found".to_string() })
    }

    #[instrument(skip(self))]
    pub async fn remove_container(&self, function_name: &str, container_id: &str) -> Result<(), LambdaError> {
        let mut containers = self.containers.lock().await;
        if let Some(container_list) = containers.get_mut(function_name) {
            container_list.retain(|container| container.container_id != container_id);
            
            // Remove empty entries
            if container_list.is_empty() {
                containers.remove(function_name);
            }
            
            info!("Removed container from warm pool: {}", container_id);
            Ok(())
        } else {
            error!("Container not found in warm pool: {}", container_id);
            Err(LambdaError::InvalidRequest { reason: "Container not found".to_string() })
        }
    }

    #[instrument(skip(self))]
    pub async fn cleanup_idle_containers(&self, soft_idle: Duration, hard_idle: Duration) -> Vec<String> {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        let mut to_stop = Vec::new();
        
        // First pass: identify containers to remove (without holding lock during removal)
        {
            let containers = self.containers.lock().await;
            for (key, container_list) in containers.iter() {
                for container in container_list.iter() {
                    let idle_time = now.duration_since(container.last_used);
                    
                    if idle_time >= hard_idle {
                        to_remove.push((key.clone(), container.container_id.clone()));
                    } else if idle_time >= soft_idle && container.is_available {
                        to_stop.push(container.container_id.clone());
                    }
                }
            }
        }
        
        // Second pass: remove hard idle containers (lock is released between iterations)
        for (key, container_id) in &to_remove {
            if let Err(e) = self.remove_container(key, container_id).await {
                error!("Failed to remove idle container {}: {}", container_id, e);
            }
        }
        
        info!("Cleaned up {} idle containers, {} to stop", to_remove.len(), to_stop.len());
        to_remove.into_iter().map(|(_, container_id)| container_id).collect()
    }

    /// Get the number of warm containers for a specific function
    pub async fn container_count(&self, function_name: &str) -> usize {
        let containers = self.containers.lock().await;
        containers.get(function_name).map(|list| list.len()).unwrap_or(0)
    }
    
    /// Get total number of warm containers across all functions
    pub async fn total_container_count(&self) -> usize {
        let containers = self.containers.lock().await;
        containers.values().map(|list| list.len()).sum()
    }
}
