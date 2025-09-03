use anyhow::Result;
use reqwest;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex, Notify};
use tokio::time::{interval, sleep};
use tracing::{debug, info, warn};
use uuid::Uuid;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

use crate::config::Config;
use crate::docker::DockerManager;
use crate::storage::FunctionStorage;

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerStatus {
    Ready,      // Container is running and available for use
    Busy,       // Container is currently executing a function
    Stopped,    // Container is stopped but can be restarted
    Draining,   // Container is being cleaned up
    Removed,    // Container has been completely removed
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub function_id: Uuid,
    pub created_at: Instant,
    pub last_used_at: Instant,
    pub invocation_count: u64,
    pub port: u16,
    pub status: ContainerStatus,
    pub config_hash: String,  // Hash of the container configuration for reuse
    pub total_execution_time_ms: u64,  // Total time spent executing functions
    pub last_execution_started: Option<Instant>,  // When the current execution started
}

#[derive(Debug, Clone)]
pub struct FunctionContainerPool {
    pub containers: Vec<ContainerInfo>,
    pub max_containers: usize,
    pub queue: VecDeque<QueuedRequest>,
}

#[derive(Debug, Clone)]
pub struct QueuedRequest {
    pub request_id: Uuid,
    pub function_id: Uuid,
    pub queued_at: Instant,
    pub notify: Arc<Notify>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub runtime: String,
    pub memory_limit: u64,
    pub cpu_limit: f64,
    pub environment_variables: HashMap<String, String>,
    pub image_name: String,
}

impl ContainerConfig {
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.runtime.as_bytes());
        hasher.update(self.memory_limit.to_le_bytes());
        hasher.update(self.cpu_limit.to_le_bytes());
        hasher.update(self.image_name.as_bytes());
        
        // Sort environment variables for consistent hashing
        let mut env_vars: Vec<_> = self.environment_variables.iter().collect();
        env_vars.sort_by_key(|(k, _)| *k);
        for (key, value) in env_vars {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }
        
        format!("{:x}", hasher.finalize())
    }
}

#[derive(Debug, Clone)]
pub struct StoppedContainer {
    pub id: String,
    pub config_hash: String,
    pub stopped_at: Instant,
    pub port: u16,
    pub invocation_count: u64,
}

pub struct ContainerLifecycleManager {
    docker_manager: DockerManager,
    config: Config,
    storage: Arc<FunctionStorage>,
    pools: Arc<RwLock<HashMap<Uuid, FunctionContainerPool>>>,
    function_locks: Arc<RwLock<HashMap<Uuid, Arc<RwLock<()>>>>>,
    cleanup_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
    container_names: Arc<RwLock<HashMap<String, String>>>, // Maps container ID to container name
    global_container_count: Arc<Mutex<usize>>, // Track total containers across all functions
    stopped_containers: Arc<RwLock<HashMap<String, Vec<StoppedContainer>>>>, // Maps config_hash to stopped containers
    max_stopped_containers: usize, // Maximum number of stopped containers to keep (default: 20)
}

impl ContainerLifecycleManager {
    // Helper function to safely slice container IDs for logging
    fn container_id_short(id: &str) -> &str {
        &id[..id.len().min(12)]
    }

    pub fn new(docker_manager: DockerManager, config: Config, storage: Arc<FunctionStorage>) -> Self {
        Self {
            docker_manager,
            config,
            storage,
            pools: Arc::new(RwLock::new(HashMap::new())),
            function_locks: Arc::new(RwLock::new(HashMap::new())),
            cleanup_task: Arc::new(RwLock::new(None)),
            container_names: Arc::new(RwLock::new(HashMap::new())),
            global_container_count: Arc::new(Mutex::new(0)),
            stopped_containers: Arc::new(RwLock::new(HashMap::new())),
            max_stopped_containers: 20, // Default limit of 20 stopped containers
        }
    }

    pub async fn start_cleanup_task(&self) {
        let pools = self.pools.clone();
        let docker_manager = self.docker_manager.clone();
        let config = self.config.clone();
        let container_names = self.container_names.clone();
        let global_count = self.global_container_count.clone();
        let stopped_containers = self.stopped_containers.clone();
        let max_stopped_containers = self.max_stopped_containers;
        
        let cleanup_interval = Duration::from_secs(config.lifecycle_config.cleanup_interval_seconds);
        let max_idle_time = Duration::from_secs(config.lifecycle_config.max_idle_time_seconds);
        let max_container_age = Duration::from_secs(config.lifecycle_config.max_container_age_seconds);

        let task = tokio::spawn(async move {
            let mut interval = interval(cleanup_interval);
            loop {
                interval.tick().await;
                
                debug!("Running container lifecycle cleanup");
                
                let mut pools_guard = pools.write().await;
                let mut containers_to_remove = Vec::new();
                let mut containers_to_stop = Vec::new();
                let mut total_removed = 0;
                
                for (function_id, pool) in pools_guard.iter_mut() {
                    let now = Instant::now();
                    let mut remove_indices = Vec::new();
                    
                    for (index, container) in pool.containers.iter().enumerate() {
                        let idle_time = now.duration_since(container.last_used_at);
                        let age = now.duration_since(container.created_at);
                        
                        // Remove containers based on status and idle time
                        match container.status {
                            ContainerStatus::Ready => {
                                // Stop and add to stopped pool if idle too long, remove if too old
                                if idle_time > max_idle_time {
                                    if age > max_container_age {
                                        // Too old, remove completely
                                        remove_indices.push(index);
                                        containers_to_remove.push((function_id.clone(), container.id.clone()));
                                    } else {
                                        // Just idle, stop and add to stopped pool
                                        remove_indices.push(index);
                                        containers_to_stop.push((function_id.clone(), container.clone()));
                                    }
                                }
                            }
                            ContainerStatus::Stopped => {
                                // Remove stopped containers that are too old
                                if age > max_container_age {
                                    remove_indices.push(index);
                                    containers_to_remove.push((function_id.clone(), container.id.clone()));
                                }
                            }
                            ContainerStatus::Draining => {
                                // Always remove draining containers during cleanup
                                remove_indices.push(index);
                                containers_to_remove.push((function_id.clone(), container.id.clone()));
                            }
                            ContainerStatus::Busy => {
                                // Only remove if extremely old
                                if age > max_container_age * 2 {
                                    remove_indices.push(index);
                                    containers_to_remove.push((function_id.clone(), container.id.clone()));
                                }
                            }
                            ContainerStatus::Removed => {
                                // Remove containers marked as removed
                                remove_indices.push(index);
                            }
                        }
                    }
                    
                    // Remove containers in reverse order to maintain indices
                    for &index in remove_indices.iter().rev() {
                        pool.containers.remove(index);
                        total_removed += 1;
                    }
                }
                
                drop(pools_guard);
                
                // Actually remove the containers
                for (function_id, container_id) in containers_to_remove {
                    info!("Cleaning up old container: {} for function: {}", container_id, function_id);
                    if let Err(e) = docker_manager.stop_container(&container_id).await {
                        warn!("Failed to stop container {}: {}", container_id, e);
                    }
                    if let Err(e) = docker_manager.remove_container(&container_id).await {
                        warn!("Failed to remove container {}: {}", container_id, e);
                    }
                    // Remove from container names map
                    let mut names_guard = container_names.write().await;
                    names_guard.remove(&container_id);
                }
                
                // Stop containers and add to stopped pool
                for (function_id, container_info) in containers_to_stop {
                    info!("Stopping idle container: {} for function: {} (adding to stopped pool)", 
                          Self::container_id_short(&container_info.id), function_id);
                    if let Err(e) = docker_manager.stop_container(&container_info.id).await {
                        warn!("Failed to stop container {}: {}", container_info.id, e);
                        // If we can't stop it, remove it completely
                        let _ = docker_manager.remove_container(&container_info.id).await;
                        let mut names_guard = container_names.write().await;
                        names_guard.remove(&container_info.id);
                    } else {
                        // Successfully stopped, add to stopped containers pool
                        // We need to create a config for this container
                        // For now, we'll use a placeholder - in a real implementation,
                        // we'd need to store the config with the container
                        let config = ContainerConfig {
                            runtime: "nodejs".to_string(), // This should be stored with the container
                            memory_limit: 128, // This should be stored with the container
                            cpu_limit: 0.5, // This should be stored with the container
                            environment_variables: HashMap::new(),
                            image_name: format!("lambda-function-{}", function_id),
                        };
                        
                        let stopped_container = StoppedContainer {
                            id: container_info.id.clone(),
                            config_hash: config.hash(),
                            stopped_at: Instant::now(),
                            port: container_info.port,
                            invocation_count: container_info.invocation_count,
                        };
                        
                        let mut stopped_containers_guard = stopped_containers.write().await;
                        let containers = stopped_containers_guard.entry(config.hash()).or_insert_with(Vec::new);
                        containers.push(stopped_container);
                        
                        // Clean up excess stopped containers
                        let mut total_stopped = 0;
                        for containers in stopped_containers_guard.values() {
                            total_stopped += containers.len();
                        }
                        
                        if total_stopped > max_stopped_containers {
                            let excess = total_stopped - max_stopped_containers;
                            info!("Cleaning up {} excess stopped containers (limit: {})", excess, max_stopped_containers);
                            
                            // Collect all stopped containers with their config hashes
                            let mut all_containers: Vec<(String, StoppedContainer)> = Vec::new();
                            for (config_hash, containers) in stopped_containers_guard.iter() {
                                for container in containers {
                                    all_containers.push((config_hash.clone(), container.clone()));
                                }
                            }
                            
                            // Sort by stopped_at (oldest first) and remove excess
                            all_containers.sort_by_key(|(_, container)| container.stopped_at);
                            
                            for (config_hash, container) in all_containers.iter().take(excess) {
                                info!("Removing excess stopped container {} (stopped {}s ago)", 
                                      Self::container_id_short(&container.id), 
                                      container.stopped_at.elapsed().as_secs());
                                
                                // Remove from Docker
                                let _ = docker_manager.remove_container(&container.id).await;
                                
                                // Remove from our tracking
                                if let Some(containers) = stopped_containers_guard.get_mut(config_hash) {
                                    containers.retain(|c| c.id != container.id);
                                    if containers.is_empty() {
                                        stopped_containers_guard.remove(config_hash);
                                    }
                                }
                            }
                        }
                    }
                }

                // Update global container count
                if total_removed > 0 {
                    let mut count_guard = global_count.lock().await;
                    *count_guard = count_guard.saturating_sub(total_removed);
                    info!("Cleaned up {} containers, global count now: {}", total_removed, *count_guard);
                }
            }
        });
        
        {
            let mut cleanup_guard = self.cleanup_task.write().await;
            *cleanup_guard = Some(task);
        }
        info!("Container lifecycle cleanup task started");
    }

    /// Get or create a per-function lock
    async fn get_function_lock(&self, function_id: &Uuid) -> Arc<RwLock<()>> {
        // First try to get existing lock
        {
            let locks_guard = self.function_locks.read().await;
            if let Some(lock) = locks_guard.get(function_id) {
                return lock.clone();
            }
        }
        
        // Create new lock if it doesn't exist
        {
            let mut locks_guard = self.function_locks.write().await;
            locks_guard.entry(*function_id).or_insert_with(|| Arc::new(RwLock::new(()))).clone()
        }
    }

    /// Get or create a container for a function, with proper queuing support
    pub async fn get_or_create_container(&self, function_id: &Uuid) -> Result<ContainerInfo> {
        // First, try to get an available container without queuing
        if let Some(container_info) = self.try_get_available_container(function_id).await? {
            return Ok(container_info);
        }

        // No available container, try to create a new one
        match self.try_create_container_without_queue(function_id).await {
            Ok(container_info) => {
                info!("Created new container {} for function {}", Self::container_id_short(&container_info.id), function_id);
                return Ok(container_info);
            }
            Err(e) => {
                // Can't create container (global limit reached), queue the request
                warn!("Cannot create container for function {}: {}. Queuing request.", function_id, e);
                return self.queue_request_and_wait(function_id).await;
            }
        }
    }

    /// Claim a specific container (mark as busy) using per-function lock
    async fn claim_container_with_lock(&self, function_id: &Uuid, container_id: &str, function_lock: &Arc<RwLock<()>>) -> Result<ContainerInfo> {
        let _lock_guard = function_lock.write().await;
        let mut pools_guard = self.pools.write().await;
        if let Some(pool) = pools_guard.get_mut(function_id) {
            for container in &mut pool.containers {
                info!("Checking container {}: status={:?}", Self::container_id_short(&container.id), container.status);
                if container.id == container_id && container.status == ContainerStatus::Ready {
                    // Atomically mark as busy and update metadata
                    container.status = ContainerStatus::Busy;
                    container.last_used_at = Instant::now();
                    container.last_execution_started = Some(Instant::now());
                    
                    let container_info = container.clone();
                    drop(pools_guard);
                    info!("✅ Claimed container {} for function {} (status: Busy)", Self::container_id_short(&container_id), function_id);
                    return Ok(container_info);
                }
            }
        }
        Err(anyhow::anyhow!("Container {} not ready for function {}", container_id, function_id))
    }

    /// Claim a specific container (mark as busy) - legacy method for compatibility
    pub async fn claim_container(&self, function_id: &Uuid, container_id: &str) -> Result<ContainerInfo> {
        let function_lock = self.get_function_lock(function_id).await;
        self.claim_container_with_lock(function_id, container_id, &function_lock).await
    }

    /// Try to create a new container, respecting global limits, using per-function lock
    async fn try_create_container_with_lock(&self, function_id: &Uuid, function_lock: &Arc<RwLock<()>>) -> Result<ContainerInfo> {
        let _lock_guard = function_lock.write().await;
        
        // Get container configuration
        let config = self.create_container_config(function_id).await?;
        let config_hash = config.hash();
        
        // First, try to get the best available container (ready or restartable)
        if let Some(container_info) = self.get_best_available_container(function_id, &config).await? {
            // Check if this container is already in the pool
            let is_already_in_pool = {
                let pools_guard = self.pools.read().await;
                if let Some(pool) = pools_guard.get(function_id) {
                    pool.containers.iter().any(|c| c.id == container_info.id)
                } else {
                    false
                }
            };
            
            if !is_already_in_pool {
                // Container is not in pool (e.g., restarted from stopped state), add it
                let mut updated_container = container_info;
                updated_container.function_id = *function_id;
                
                let mut pools_guard = self.pools.write().await;
                let pool = pools_guard.entry(*function_id).or_insert_with(|| FunctionContainerPool {
                    containers: Vec::new(),
                    max_containers: self.config.lifecycle_config.max_containers_per_function,
                    queue: VecDeque::new(),
                });
                
                pool.containers.push(updated_container.clone());
                drop(pools_guard);
                
                info!("Added restarted container {} to pool for function {} (global count: {})", 
                      Self::container_id_short(&updated_container.id), function_id, *self.global_container_count.lock().await);
                
                return Ok(updated_container);
            } else {
                // Container is already in pool, just return it
                info!("Using existing container {} for function {} (global count: {})", 
                      Self::container_id_short(&container_info.id), function_id, *self.global_container_count.lock().await);
                
                return Ok(container_info);
            }
        }
        
        // No available container, proceed with creating a new one
        info!("No available container for function {}, creating new container", function_id);
        
        // Check global container limit with minimal lock time
        let can_create = {
            let global_count_guard = self.global_container_count.lock().await;
            *global_count_guard < self.config.lifecycle_config.max_global_containers
        };

        if !can_create {
            // Use smart eviction to make room
            if let Some(evicted_container) = self.smart_eviction(&config_hash).await? {
                info!("Smart eviction: Demoted container {} to make room for new container", Self::container_id_short(&evicted_container.id));
                // The evicted container is now in stopped state, we can proceed with creation
            } else {
                return Err(anyhow::anyhow!("Global container limit reached and no containers to evict"));
            }
        }

        // Check per-function limit with minimal lock time
        let needs_cleanup = {
            let pools_guard = self.pools.read().await;
            if let Some(pool) = pools_guard.get(function_id) {
                pool.containers.len() >= self.config.lifecycle_config.max_containers_per_function
            } else {
                false
            }
        };

        if needs_cleanup {
            // Remove the oldest container for this function
            let oldest_container_id = {
                let mut pools_guard = self.pools.write().await;
                if let Some(pool) = pools_guard.get_mut(function_id) {
                    if let Some(oldest_container) = pool.containers.first() {
                        let container_id = oldest_container.id.clone();
                        // Remove from pool
                        pool.containers.remove(0);
                        Some(container_id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(container_id) = oldest_container_id {
                info!("Removing oldest container {} to make room for new one", container_id);
                let _ = self.docker_manager.stop_container(&container_id).await;
                let _ = self.docker_manager.remove_container(&container_id).await;
                
                // Update global count
                let mut global_count_guard = self.global_container_count.lock().await;
                *global_count_guard = global_count_guard.saturating_sub(1);
            }
        }

        // Get function details to determine memory and CPU limits
        let function = self.storage
            .get_function(function_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Function not found: {}", function_id))?;

        let memory_limit = function.memory_size.unwrap_or(128) as u64; // Default to 128 MB
        let cpu_limit = function.cpu_limit.unwrap_or(0.5); // Default to 0.5 CPU

        info!("Creating container for function {} with {} MB memory and {} CPU", 
              function_id, memory_limit, cpu_limit);

        // Create new container
        let container_name = format!("lambda-{}-{}", function_id, Uuid::new_v4());
        let container_id = self.docker_manager
            .create_container(
                function_id,
                &container_name,
                &std::collections::HashMap::new(),
                memory_limit as u64,
                cpu_limit as f64,
            )
            .await?;

        // Store container name mapping
        let mut names_guard = self.container_names.write().await;
        names_guard.insert(container_id.clone(), container_name.clone());

        self.docker_manager.start_container(&container_id).await?;
        
        // Wait for container to be ready and get port
        sleep(Duration::from_secs(2)).await;
        let port = self.docker_manager.get_container_mapped_port(&container_id, 8080).await? as u16;

        let container_info = ContainerInfo {
            id: container_id,
            function_id: *function_id,
            created_at: Instant::now(),
            last_used_at: Instant::now(),
            invocation_count: 0,
            port,
            status: ContainerStatus::Busy, // Mark as busy immediately
            config_hash: config_hash.clone(),
            total_execution_time_ms: 0,
            last_execution_started: Some(Instant::now()),
        };

        // Add to pool and update global count
        {
            let mut pools_guard = self.pools.write().await;
            let pool = pools_guard.entry(*function_id).or_insert_with(|| FunctionContainerPool {
                containers: Vec::new(),
                max_containers: self.config.lifecycle_config.max_containers_per_function,
                queue: VecDeque::new(),
            });
            
            pool.containers.push(container_info.clone());
        }
        
        // Update global count
        {
            let mut global_count_guard = self.global_container_count.lock().await;
            *global_count_guard += 1;
            info!("Created new container {} for function {} on port {} (global count: {})", 
                  container_info.id, function_id, container_info.port, *global_count_guard);
        }

        Ok(container_info)
    }

    /// Try to create a new container, respecting global limits - legacy method for compatibility
    pub async fn try_create_container(&self, function_id: &Uuid) -> Result<ContainerInfo> {
        let function_lock = self.get_function_lock(function_id).await;
        self.try_create_container_with_lock(function_id, &function_lock).await
    }

    /// Smart eviction strategy to make room for new containers
    async fn smart_eviction(&self, _target_config_hash: &str) -> Result<Option<ContainerInfo>> {
        // For now, just return None - we'll implement this later if needed
        Ok(None)
    }

    /// Check if RIC is ready via Lambda Runtime API health endpoint
    async fn is_ric_ready(&self, container_id: &str) -> Result<bool> {
        let host_port = self.docker_manager.get_container_mapped_port(container_id, 8080).await?;
        let health_url = format!("http://localhost:{}/health", host_port);
        
        // Use tokio::time::timeout to ensure we don't hang forever
        match tokio::time::timeout(Duration::from_millis(1000), reqwest::get(&health_url)).await {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            Ok(Err(_)) | Err(_) => {
                // Timeout or connection error - assume not ready
                debug!("RIC health check failed for container {}: timeout or connection error", Self::container_id_short(container_id));
                Ok(false)
            }
        }
    }

    /// Get the best available container for a function
    pub async fn get_best_available_container(&self, function_id: &Uuid, _config: &ContainerConfig) -> Result<Option<ContainerInfo>> {
        // Just try to get an available container
        self.try_get_available_container(function_id).await
    }

    /// Try to get an available container without queuing
    async fn try_get_available_container(&self, function_id: &Uuid) -> Result<Option<ContainerInfo>> {
        let pools_guard = self.pools.read().await;
        if let Some(pool) = pools_guard.get(function_id) {
            // Find an available container
            for container in &pool.containers {
                if container.status == ContainerStatus::Ready {
                    // Check RIC health endpoint - RIC is the source of truth for execution state
                    if self.is_ric_ready(&container.id).await? {
                        return self.claim_container_atomically(function_id, &container.id).await;
                    } else {
                        warn!("Container {} marked as Ready but RIC not responding, skipping", Self::container_id_short(&container.id));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Try to create a container without queuing (respects limits)
    async fn try_create_container_without_queue(&self, function_id: &Uuid) -> Result<ContainerInfo> {
        // Check global container limit
        let can_create = {
            let global_count_guard = self.global_container_count.lock().await;
            *global_count_guard < self.config.lifecycle_config.max_global_containers
        };

        if !can_create {
            return Err(anyhow::anyhow!("Global container limit reached"));
        }

        // Check per-function limit
        let needs_cleanup = {
            let pools_guard = self.pools.read().await;
            if let Some(pool) = pools_guard.get(function_id) {
                pool.containers.len() >= self.config.lifecycle_config.max_containers_per_function
            } else {
                false
            }
        };

        if needs_cleanup {
            return Err(anyhow::anyhow!("Per-function container limit reached"));
        }

        // Create the container
        self.create_new_container(function_id).await
    }

    /// Queue a request and wait for it to be processed
    async fn queue_request_and_wait(&self, function_id: &Uuid) -> Result<ContainerInfo> {
        let request_id = Uuid::new_v4();
        let notify = Arc::new(Notify::new());
        
        // Add to queue
        {
            let mut pools_guard = self.pools.write().await;
            let pool = pools_guard.entry(*function_id).or_insert_with(|| FunctionContainerPool {
                containers: Vec::new(),
                max_containers: self.config.lifecycle_config.max_containers_per_function,
                queue: VecDeque::new(),
            });
            
            pool.queue.push_back(QueuedRequest {
                request_id,
                function_id: *function_id,
                queued_at: Instant::now(),
                notify: notify.clone(),
            });
            
            info!("Queued request {} for function {} (queue size: {})", 
                  request_id, function_id, pool.queue.len());
        }

        // Start the queue processor if it's not already running
        self.process_queue(function_id).await;

        // Wait for a container to become available
        info!("Waiting for container to become available for function {}", function_id);
        notify.notified().await;

        // The queue processor should have provided us with a container
        // Try to get it from the pool
        self.get_available_container_after_queue(function_id).await
    }

    /// Claim a container atomically (without locks)
    async fn claim_container_atomically(&self, function_id: &Uuid, container_id: &str) -> Result<Option<ContainerInfo>> {
        let mut pools_guard = self.pools.write().await;
        if let Some(pool) = pools_guard.get_mut(function_id) {
            for container in &mut pool.containers {
                if container.id == container_id && container.status == ContainerStatus::Ready {
                    // Atomically mark as busy
                    container.status = ContainerStatus::Busy;
                    container.last_used_at = Instant::now();
                    container.last_execution_started = Some(Instant::now());
                    
                    let container_info = container.clone();
                    drop(pools_guard);
                    info!("✅ Claimed container {} for function {} (status: Busy)", Self::container_id_short(&container_id), function_id);
                    return Ok(Some(container_info));
                }
            }
        }
        Ok(None)
    }

    /// Create a new container (extracted from the complex method)
    async fn create_new_container(&self, function_id: &Uuid) -> Result<ContainerInfo> {
        // Get container configuration
        let config = self.create_container_config(function_id).await?;
        let config_hash = config.hash();
        
        // Get function details to determine memory and CPU limits
        let function = self.storage
            .get_function(function_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Function not found: {}", function_id))?;

        let memory_limit = function.memory_size.unwrap_or(128) as u64;
        let cpu_limit = function.cpu_limit.unwrap_or(0.5);

        info!("Creating container for function {} with {} MB memory and {} CPU", 
              function_id, memory_limit, cpu_limit);

        // Create new container
        let container_name = format!("lambda-{}-{}", function_id, Uuid::new_v4());
        let container_id = self.docker_manager
            .create_container(
                function_id,
                &container_name,
                &std::collections::HashMap::new(),
                memory_limit as u64,
                cpu_limit as f64,
            )
            .await?;

        // Store container name mapping
        let mut names_guard = self.container_names.write().await;
        names_guard.insert(container_id.clone(), container_name.clone());

        self.docker_manager.start_container(&container_id).await?;
        
        // Wait for container to be ready and get port
        sleep(Duration::from_secs(2)).await;
        let port = self.docker_manager.get_container_mapped_port(&container_id, 8080).await? as u16;

        let container_info = ContainerInfo {
            id: container_id,
            function_id: *function_id,
            created_at: Instant::now(),
            last_used_at: Instant::now(),
            invocation_count: 0,
            port,
            status: ContainerStatus::Busy, // Mark as busy immediately
            config_hash: config_hash.clone(),
            total_execution_time_ms: 0,
            last_execution_started: Some(Instant::now()),
        };

        // Add to pool and update global count
        {
            let mut pools_guard = self.pools.write().await;
            let pool = pools_guard.entry(*function_id).or_insert_with(|| FunctionContainerPool {
                containers: Vec::new(),
                max_containers: self.config.lifecycle_config.max_containers_per_function,
                queue: VecDeque::new(),
            });
            
            pool.containers.push(container_info.clone());
        }
        
        // Update global count
        {
            let mut global_count_guard = self.global_container_count.lock().await;
            *global_count_guard += 1;
            info!("Created new container {} for function {} on port {} (global count: {})", 
                  container_info.id, function_id, container_info.port, *global_count_guard);
        }

        Ok(container_info)
    }



    /// Get an available container after queuing (used by the waiting request)
    async fn get_available_container_after_queue(&self, function_id: &Uuid) -> Result<ContainerInfo> {
        let pools_guard = self.pools.read().await;
        if let Some(pool) = pools_guard.get(function_id) {
            // Find an available container
            for container in &pool.containers {
                if container.status == ContainerStatus::Ready {
                    // Trust our internal state - the RIC handles execution state
                    if let Some(container_info) = self.claim_container_atomically(function_id, &container.id).await? {
                        return Ok(container_info);
                    }
                }
            }
        }
        
        // If no available container, this is an error (should not happen after queuing)
        Err(anyhow::anyhow!("No available container found after queuing for function {}", function_id))
    }

    /// Process queued requests when a container becomes available (simplified)
    async fn process_queue(&self, function_id: &Uuid) {
        // Just notify the first queued request - let it handle getting the container
        let mut pools_guard = self.pools.write().await;
        if let Some(pool) = pools_guard.get_mut(function_id) {
            if let Some(queued_request) = pool.queue.pop_front() {
                info!("Notifying queued request {} for function {}", 
                      queued_request.request_id, function_id);
                queued_request.notify.notify_one();
            }
        }
    }

    pub async fn mark_container_available(&self, function_id: &Uuid, container_id: &str) -> Result<()> {
        info!("🔄 Marking container {} as available for function {}", Self::container_id_short(&container_id), function_id);
        let mut pools_guard = self.pools.write().await;
        if let Some(pool) = pools_guard.get_mut(function_id) {
            for container in &mut pool.containers {
                if container.id == container_id {
                    info!("Found container {} with status {:?}", Self::container_id_short(&container_id), container.status);
                    // Update execution time if we have a start time
                    if let Some(start_time) = container.last_execution_started {
                        let execution_duration = start_time.elapsed();
                        container.total_execution_time_ms += execution_duration.as_millis() as u64;
                        container.last_execution_started = None;
                        info!("Updated execution time: {}ms", execution_duration.as_millis());
                    }
                    
                    container.status = ContainerStatus::Ready;
                    container.last_used_at = Instant::now();
                    container.invocation_count += 1;
                    info!("✅ Marked container {} as ready for function {} (invocation count: {})", Self::container_id_short(&container_id), function_id, container.invocation_count);
                    break;
                }
            }
        } else {
            info!("❌ No pool found for function {} when marking container available", function_id);
        }
        drop(pools_guard);

        // Process any queued requests
        self.process_queue(function_id).await;

        Ok(())
    }

    pub async fn get_container_stats(&self) -> Result<HashMap<Uuid, FunctionContainerPool>> {
        let pools_guard = self.pools.read().await;
        Ok(pools_guard.clone())
    }

    pub async fn get_lifecycle_stats(&self) -> Result<HashMap<Uuid, FunctionContainerPool>> {
        self.get_container_stats().await
    }

    pub async fn get_global_stats(&self) -> Result<(usize, usize)> {
        let global_count = *self.global_container_count.lock().await;
        let max_global = self.config.lifecycle_config.max_global_containers;
        Ok((global_count, max_global))
    }

    /// Create a container configuration hash for a function
    async fn create_container_config(&self, function_id: &Uuid) -> Result<ContainerConfig> {
        let function = self.storage
            .get_function(function_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Function not found: {}", function_id))?;

        let memory_limit = function.memory_size.unwrap_or(128) as u64;
        let cpu_limit = function.cpu_limit.unwrap_or(0.5);
        let image_name = format!("lambda-function-{}", function_id);

        Ok(ContainerConfig {
            runtime: function.runtime.clone(),
            memory_limit,
            cpu_limit,
            environment_variables: function.environment.unwrap_or_default(),
            image_name,
        })
    }







    /// Clean up all containers (used during shutdown)
    pub async fn cleanup_all_containers(&self) {
        let pools_guard = self.pools.write().await;
        
        for (function_id, pool) in pools_guard.iter() {
            for container in &pool.containers {
                let container_id = &container.id;
                info!("Shutting down container: {} for function: {}", container_id, function_id);
                if let Err(e) = self.docker_manager.stop_container(container_id).await {
                    warn!("Failed to stop container {}: {}", container_id, e);
                }
                if let Err(e) = self.docker_manager.remove_container(container_id).await {
                    warn!("Failed to remove container {}: {}", container_id, e);
                }
            }
        }

        // Clear container names map
        let mut names_guard = self.container_names.write().await;
        names_guard.clear();

        // Reset global count
        let mut global_count_guard = self.global_container_count.lock().await;
        *global_count_guard = 0;

        info!("All containers cleaned up");
    }
}
