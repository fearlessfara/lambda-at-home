use std::collections::HashMap;
use uuid::Uuid;
use lambda_models::{Function, LambdaError};
use crate::docker::Invoker;
use tracing::{info, error, instrument};

pub struct ContainerManager {
    invoker: Invoker,
    active_containers: HashMap<String, ContainerInfo>,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub container_id: String,
    pub function_id: Uuid,
    pub created_at: std::time::Instant,
    pub last_used: std::time::Instant,
    pub status: ContainerStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerStatus {
    Starting,
    Running,
    Stopped,
    Failed,
    Removed,
}

impl ContainerManager {
    pub async fn new(invoker: Invoker) -> Result<Self, LambdaError> {
        Ok(Self {
            invoker,
            active_containers: HashMap::new(),
        })
    }

    #[instrument(skip(self))]
    pub async fn create_and_start_container(
        &mut self,
        function: &Function,
        image_ref: &str,
        env_vars: HashMap<String, String>,
    ) -> Result<String, LambdaError> {
        let container_id = self.invoker.create_container(function, image_ref, env_vars).await?;
        
        let container_info = ContainerInfo {
            container_id: container_id.clone(),
            function_id: function.function_id,
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
            status: ContainerStatus::Starting,
        };
        
        self.active_containers.insert(container_id.clone(), container_info);
        
        self.invoker.start_container(&container_id).await?;
        
        // Update status to running
        if let Some(info) = self.active_containers.get_mut(&container_id) {
            info.status = ContainerStatus::Running;
        }
        
        info!("Created and started container: {} for function: {}", container_id, function.function_name);
        Ok(container_id)
    }

    #[instrument(skip(self))]
    pub async fn stop_container(&mut self, container_id: &str) -> Result<(), LambdaError> {
        self.invoker.stop_container(container_id).await?;
        
        if let Some(info) = self.active_containers.get_mut(container_id) {
            info.status = ContainerStatus::Stopped;
            info.last_used = std::time::Instant::now();
        }
        
        info!("Stopped container: {}", container_id);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn remove_container(&mut self, container_id: &str) -> Result<(), LambdaError> {
        self.invoker.remove_container(container_id).await?;
        
        if let Some(info) = self.active_containers.get_mut(container_id) {
            info.status = ContainerStatus::Removed;
        }
        
        self.active_containers.remove(container_id);
        
        info!("Removed container: {}", container_id);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_container_logs(&self, container_id: &str) -> Result<String, LambdaError> {
        self.invoker.get_container_logs(container_id).await
    }

    #[instrument(skip(self))]
    pub async fn wait_for_container_completion(
        &self,
        container_id: &str,
        timeout_ms: u64,
    ) -> Result<i64, LambdaError> {
        self.invoker.wait_for_container(container_id, timeout_ms).await
    }

    #[instrument(skip(self))]
    pub fn get_container_info(&self, container_id: &str) -> Option<&ContainerInfo> {
        self.active_containers.get(container_id)
    }

    #[instrument(skip(self))]
    pub fn get_containers_for_function(&self, function_id: Uuid) -> Vec<&ContainerInfo> {
        self.active_containers
            .values()
            .filter(|info| info.function_id == function_id)
            .collect()
    }

    #[instrument(skip(self))]
    pub fn mark_container_failed(&mut self, container_id: &str) {
        if let Some(info) = self.active_containers.get_mut(container_id) {
            info.status = ContainerStatus::Failed;
            error!("Marked container as failed: {}", container_id);
        }
    }

    #[instrument(skip(self))]
    pub fn update_container_last_used(&mut self, container_id: &str) {
        if let Some(info) = self.active_containers.get_mut(container_id) {
            info.last_used = std::time::Instant::now();
        }
    }

    #[instrument(skip(self))]
    pub fn get_idle_containers(&self, idle_duration: std::time::Duration) -> Vec<String> {
        let now = std::time::Instant::now();
        self.active_containers
            .iter()
            .filter(|(_, info)| {
                info.status == ContainerStatus::Running &&
                now.duration_since(info.last_used) >= idle_duration
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
}
