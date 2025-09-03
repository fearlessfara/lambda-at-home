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
    pending: Arc<Pending>,
    invoker: Arc<Invoker>,
}

impl IdleWatchdog {
    pub fn new(config: Config, warm_pool: Arc<WarmPool>, pending: Arc<Pending>, invoker: Arc<Invoker>) -> Self {
        Self {
            config,
            warm_pool,
            pending,
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
        
        // Get containers to remove (hard idle)
        let containers_to_remove = self.warm_pool.cleanup_idle_containers(soft_idle, hard_idle).await;
        
        // Remove containers from Docker
        for container_id in containers_to_remove {
            if let Err(e) = self.invoker.remove_container(&container_id).await {
                error!("Failed to remove idle container {}: {}", container_id, e);
            }
        }
        
        // TODO: Also stop containers that are soft idle but not hard idle
        // This would involve calling invoker.stop_container() for containers
        // that have been idle for soft_idle duration but not hard_idle duration
        
        Ok(())
    }
}
