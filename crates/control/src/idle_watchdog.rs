use std::time::Duration;
use std::sync::Arc;
use tokio::time::interval;

use lambda_models::Config;
use crate::warm_pool::WarmPool;
use crate::pending::Pending;
use lambda_invoker::Invoker;
use tracing::{error, instrument, info};

pub struct IdleWatchdog {
    config: Config,
    warm_pool: Arc<WarmPool>,
    _pending: Arc<Pending>,
    invoker: Arc<Invoker>,
}

impl IdleWatchdog {
    pub fn new(config: Config, warm_pool: Arc<WarmPool>, pending: Arc<Pending>, invoker: Arc<Invoker>) -> Self {
        Self {
            config,
            warm_pool,
            _pending: pending,
            invoker,
        }
    }

    #[instrument(skip(self))]
    pub async fn start(&self) {
        info!("Starting idle watchdog with soft_idle: {}ms, hard_idle: {}ms", 
              self.config.idle.soft_ms, self.config.idle.hard_ms);
        
        let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.run_cleanup_cycle().await {
                error!("Idle watchdog cleanup cycle failed: {}", e);
            }
        }
    }

    #[instrument(skip(self))]
    async fn run_cleanup_cycle(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let soft_idle = Duration::from_millis(self.config.idle.soft_ms);
        let hard_idle = Duration::from_millis(self.config.idle.hard_ms);
        
        // Identify hard idle (remove) and perform removal from pool
        let containers_to_remove = self.warm_pool.cleanup_idle_containers(soft_idle, hard_idle).await;
        
        // Remove containers from Docker
        for container_id in containers_to_remove {
            if let Err(e) = self.invoker.remove_container(&container_id).await {
                error!("Failed to remove idle container {}: {}", container_id, e);
            }
        }
        
        // Also stop containers that are soft idle but not hard idle (best effort)
        let soft_idle_list = self.warm_pool.list_soft_idle_containers(soft_idle).await;
        for container_id in soft_idle_list {
            // Mark as stopping in pool so scheduler doesn't consider it available
            let _ = self.warm_pool.set_state_by_container_id(&container_id, crate::warm_pool::InstanceState::Stopping).await;
            match self.invoker.stop_container(&container_id).await {
                Ok(_) => {
                    // Mark as fully stopped now
                    let _ = self.warm_pool.set_state_by_container_id(&container_id, crate::warm_pool::InstanceState::Stopped).await;
                    info!("Soft-idle container stopped: {}", container_id);
                }
                Err(e) => {
                    error!("Failed to stop soft-idle container {}: {}", container_id, e);
                }
            }
        }
        
        Ok(())
    }
}
