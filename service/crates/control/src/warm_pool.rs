use crate::queues::FnKey;
use dashmap::DashMap;
use lambda_models::LambdaError;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, instrument};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct WarmContainer {
    pub container_id: String,
    pub instance_id: String,
    pub function_id: Uuid,
    pub image_ref: String,
    pub created_at: Instant,
    pub last_used: Instant,
    pub state: InstanceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    Init,
    Provisioning,
    Initializing,
    WarmIdle,
    Active,
    Draining,
    Stopping,
    Stopped,
    Terminated,
    Failed,
}

#[derive(Clone)]
pub struct WarmPool {
    // Key by FnKey for proper isolation (function+runtime+version+env)
    // Using DashMap for lock-free concurrent access instead of Mutex<HashMap>
    containers: Arc<DashMap<FnKey, Vec<WarmContainer>>>,
}

impl Default for WarmPool {
    fn default() -> Self {
        Self::new()
    }
}

impl WarmPool {
    pub fn new() -> Self {
        Self {
            containers: Arc::new(DashMap::new()),
        }
    }

    #[instrument(skip(self))]
    pub async fn get_warm_container(&self, key: &FnKey) -> Option<WarmContainer> {
        if let Some(mut container_list) = self.containers.get_mut(key) {
            // Find first available container for this function
            for container in container_list.iter_mut() {
                if container.state == InstanceState::WarmIdle {
                    container.state = InstanceState::Active;
                    container.last_used = Instant::now();

                    info!(
                        "Reusing warm container: {} for function: {}",
                        container.container_id, key.function_name
                    );
                    return Some(container.clone());
                }
            }
        }

        None
    }

    /// Mark one idle container as Active for the given key. Returns its ID.
    pub async fn mark_one_active(&self, key: &FnKey) -> Option<String> {
        if let Some(mut list) = self.containers.get_mut(key) {
            for c in list.iter_mut() {
                if c.state == InstanceState::WarmIdle {
                    c.state = InstanceState::Active;
                    c.last_used = Instant::now();
                    return Some(c.container_id.clone());
                }
            }
        }
        None
    }

    /// Mark one active container as WarmIdle for the given key. Returns its ID.
    pub async fn mark_one_idle(&self, key: &FnKey) -> Option<String> {
        if let Some(mut list) = self.containers.get_mut(key) {
            for c in list.iter_mut() {
                if c.state == InstanceState::Active {
                    c.state = InstanceState::WarmIdle;
                    c.last_used = Instant::now();
                    return Some(c.container_id.clone());
                }
            }
        }
        None
    }

    /// Fallback: mark any one Active container back to WarmIdle across all keys.
    pub async fn mark_any_active_to_idle(&self) -> Option<(FnKey, String)> {
        // Collect keys first to avoid nested locks
        let keys: Vec<FnKey> = self
            .containers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys {
            if let Some(mut list) = self.containers.get_mut(&key) {
                if let Some(c_mut) = list.iter_mut().find(|c| c.state == InstanceState::Active) {
                    c_mut.state = InstanceState::WarmIdle;
                    c_mut.last_used = Instant::now();
                    return Some((key, c_mut.container_id.clone()));
                }
            }
        }
        None
    }

    #[instrument(skip(self))]
    pub async fn add_warm_container(&self, key: FnKey, container: WarmContainer) {
        let container_id = container.container_id.clone();

        self.containers
            .entry(key.clone())
            .or_default()
            .push(container);

        info!(
            "Added warm container: {} for function: {}",
            container_id, key.function_name
        );
    }

    #[instrument(skip(self))]
    pub async fn return_container(
        &self,
        key: &FnKey,
        container_id: &str,
    ) -> Result<(), LambdaError> {
        if let Some(mut container_list) = self.containers.get_mut(key) {
            for container in container_list.iter_mut() {
                if container.container_id == container_id {
                    container.state = InstanceState::WarmIdle;
                    container.last_used = Instant::now();
                    info!("Returned container to warm pool: {}", container_id);
                    return Ok(());
                }
            }
        }

        error!("Container not found in warm pool: {}", container_id);
        Err(LambdaError::InvalidRequest {
            reason: "Container not found".to_string(),
        })
    }

    #[instrument(skip(self))]
    pub async fn remove_container(
        &self,
        key: &FnKey,
        container_id: &str,
    ) -> Result<(), LambdaError> {
        if let Some(mut container_list) = self.containers.get_mut(key) {
            container_list.retain(|container| container.container_id != container_id);

            // Determine emptiness, then drop guard before removing key
            let now_empty = container_list.is_empty();
            drop(container_list);

            if now_empty {
                self.containers.remove(key);
            }

            info!("Removed container from warm pool: {}", container_id);
            Ok(())
        } else {
            error!("Container not found in warm pool: {}", container_id);
            Err(LambdaError::InvalidRequest {
                reason: "Container not found".to_string(),
            })
        }
    }

    #[instrument(skip(self))]
    pub async fn cleanup_idle_containers(
        &self,
        soft_idle: Duration,
        hard_idle: Duration,
    ) -> Vec<String> {
        let now = Instant::now();
        let mut to_remove = Vec::new();
        let mut to_stop = Vec::new();

        // First pass: identify containers to remove (without holding lock during removal)
        for entry in self.containers.iter() {
            let key = entry.key().clone();
            for container in entry.value().iter() {
                let idle_time = now.duration_since(container.last_used);

                if idle_time >= hard_idle {
                    to_remove.push((key.clone(), container.container_id.clone()));
                } else if idle_time >= soft_idle && container.state == InstanceState::WarmIdle {
                    to_stop.push(container.container_id.clone());
                }
            }
        }

        // Second pass: remove hard idle containers (lock is released between iterations)
        for (key, container_id) in &to_remove {
            if let Err(e) = self.remove_container(key, container_id).await {
                error!("Failed to remove idle container {}: {}", container_id, e);
            }
        }

        info!(
            "Cleaned up {} idle containers, {} to stop",
            to_remove.len(),
            to_stop.len()
        );
        to_remove
            .into_iter()
            .map(|(_, container_id)| container_id)
            .collect()
    }

    /// Get the number of warm containers for a specific function
    pub async fn container_count(&self, key: &FnKey) -> usize {
        self.containers.get(key).map(|list| list.len()).unwrap_or(0)
    }

    /// Get total number of warm containers across all functions
    pub async fn total_container_count(&self) -> usize {
        self.containers
            .iter()
            .map(|entry| entry.value().len())
            .sum()
    }

    /// Check if there is at least one idle (WarmIdle) container for the key
    pub async fn has_available(&self, key: &FnKey) -> bool {
        if let Some(list) = self.containers.get(key) {
            list.iter().any(|c| c.state == InstanceState::WarmIdle)
        } else {
            false
        }
    }

    /// List containers that are soft idle (idle beyond soft_idle and currently WarmIdle)
    pub async fn list_soft_idle_containers(&self, soft_idle: Duration) -> Vec<String> {
        let now = Instant::now();
        let mut to_stop = Vec::new();
        for entry in self.containers.iter() {
            for c in entry.value().iter() {
                let idle_time = now.duration_since(c.last_used);
                if idle_time >= soft_idle && c.state == InstanceState::WarmIdle {
                    to_stop.push(c.container_id.clone());
                }
            }
        }
        to_stop
    }

    /// Count containers in a specific state for a key
    pub async fn count_state(&self, key: &FnKey, state: InstanceState) -> usize {
        self.containers
            .get(key)
            .map(|list| list.iter().filter(|c| c.state == state).count())
            .unwrap_or(0)
    }

    /// List stopped container IDs for a key
    pub async fn list_stopped(&self, key: &FnKey) -> Vec<String> {
        self.containers
            .get(key)
            .map(|list| {
                list.iter()
                    .filter(|c| c.state == InstanceState::Stopped)
                    .map(|c| c.container_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Set container state by container_id across all keys.
    pub async fn set_state_by_container_id(
        &self,
        container_id: &str,
        state: InstanceState,
    ) -> bool {
        // Collect keys first to avoid nested locking during mutation
        let keys: Vec<FnKey> = self
            .containers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys {
            if let Some(mut list) = self.containers.get_mut(&key) {
                if let Some(c_mut) = list.iter_mut().find(|c| c.container_id == container_id) {
                    c_mut.state = state;
                    return true;
                }
            }
        }
        false
    }

    /// Get one stopped container for a given key if present (does not change state)
    pub async fn get_one_stopped(&self, key: &FnKey) -> Option<String> {
        if let Some(list) = self.containers.get(key) {
            for c in list.iter() {
                if c.state == InstanceState::Stopped {
                    return Some(c.container_id.clone());
                }
            }
        }
        None
    }

    /// Drain all containers from the pool and return their container IDs (for shutdown cleanup)
    pub async fn drain_all(&self) -> Vec<String> {
        let mut ids = Vec::new();
        for entry in self.containers.iter() {
            for c in entry.value().iter() {
                ids.push(c.container_id.clone());
            }
        }
        self.containers.clear();
        ids
    }

    /// Remove all containers belonging to a given function_id across all keys.
    /// Returns the list of container IDs removed.
    pub async fn drain_by_function_id(&self, function_id: Uuid) -> Vec<String> {
        let mut removed: Vec<String> = Vec::new();

        // Collect all keys first to avoid modifying while iterating
        let keys: Vec<FnKey> = self
            .containers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys {
            let mut now_empty = false;
            if let Some(mut list) = self.containers.get_mut(&key) {
                // Use retain to filter out matching containers
                list.retain(|c| {
                    if c.function_id == function_id {
                        removed.push(c.container_id.clone());
                        false // Remove this item
                    } else {
                        true // Keep this item
                    }
                });
                now_empty = list.is_empty();
            }

            if now_empty {
                // Remove key after dropping the guard
                self.containers.remove(&key);
            }
        }
        removed
    }

    /// Mark a specific instance by its instance_id as Active
    pub async fn mark_active_by_instance(&self, instance_id: &str) -> Option<(FnKey, String)> {
        // Collect keys first to avoid nested locking
        let keys: Vec<FnKey> = self
            .containers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys {
            if let Some(mut list) = self.containers.get_mut(&key) {
                if let Some(c_mut) = list.iter_mut().find(|c| c.instance_id == instance_id) {
                    c_mut.state = InstanceState::Active;
                    c_mut.last_used = Instant::now();
                    return Some((key, c_mut.container_id.clone()));
                }
            }
        }
        None
    }

    /// Mark a specific instance by its instance_id as WarmIdle
    pub async fn mark_idle_by_instance(&self, instance_id: &str) -> Option<(FnKey, String)> {
        // Collect keys first to avoid nested locking
        let keys: Vec<FnKey> = self
            .containers
            .iter()
            .map(|entry| entry.key().clone())
            .collect();

        for key in keys {
            if let Some(mut list) = self.containers.get_mut(&key) {
                if let Some(c_mut) = list.iter_mut().find(|c| c.instance_id == instance_id) {
                    c_mut.state = InstanceState::WarmIdle;
                    c_mut.last_used = Instant::now();
                    return Some((key, c_mut.container_id.clone()));
                }
            }
        }
        None
    }

    /// Build a summary for a given function name across all keys (versions/envs).
    pub async fn summary_for_function(&self, function_name: &str) -> WarmPoolSummary {
        let now = Instant::now();
        let mut warm_idle = 0usize;
        let mut active = 0usize;
        let mut stopped = 0usize;
        let mut entries = Vec::new();
        for entry in self.containers.iter() {
            let key = entry.key();
            if key.function_name != function_name {
                continue;
            }
            for c in entry.value().iter() {
                match c.state {
                    InstanceState::WarmIdle => warm_idle += 1,
                    InstanceState::Active => active += 1,
                    InstanceState::Stopped => stopped += 1,
                    _ => {}
                }
                entries.push(WarmPoolEntry {
                    container_id: c.container_id.clone(),
                    state: format!("{:?}", c.state),
                    idle_for_ms: now.saturating_duration_since(c.last_used).as_millis() as u64,
                });
            }
        }
        WarmPoolSummary {
            total: warm_idle + active + stopped,
            warm_idle,
            active,
            stopped,
            entries,
        }
    }
}

#[derive(serde::Serialize)]
pub struct WarmPoolSummary {
    pub total: usize,
    pub warm_idle: usize,
    pub active: usize,
    pub stopped: usize,
    pub entries: Vec<WarmPoolEntry>,
}

#[derive(serde::Serialize)]
pub struct WarmPoolEntry {
    pub container_id: String,
    pub state: String,
    pub idle_for_ms: u64,
}
