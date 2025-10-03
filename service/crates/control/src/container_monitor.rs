use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::warm_pool::{InstanceState, WarmPool};
use lambda_invoker::{ContainerEvent, ContainerEventSender};
use tracing::{error, info, instrument, warn};

pub struct ContainerMonitor {
    warm_pool: Arc<WarmPool>,
    event_receiver: mpsc::UnboundedReceiver<ContainerEvent>,
}

impl ContainerMonitor {
    pub fn new(warm_pool: Arc<WarmPool>) -> (Self, ContainerEventSender) {
        let (sender, receiver) = mpsc::unbounded_channel();

        let monitor = Self {
            warm_pool,
            event_receiver: receiver,
        };

        (monitor, sender)
    }

    #[instrument(skip(self))]
    pub async fn start(mut self) {
        info!("Starting container state monitor");

        // Also start a periodic sync to catch any missed events
        let warm_pool = self.warm_pool.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5)); // Check every 5 seconds

            loop {
                interval.tick().await;
                if let Err(e) = Self::sync_container_states(&warm_pool).await {
                    error!("Container state sync failed: {}", e);
                }
            }
        });

        // Process events from Docker
        while let Some(event) = self.event_receiver.recv().await {
            if let Err(e) = self.handle_container_event(event).await {
                error!("Failed to handle container event: {}", e);
            }
        }

        warn!("Container state monitor stopped");
    }

    pub async fn handle_container_event(
        &self,
        event: ContainerEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match event {
            ContainerEvent::Die {
                container_id,
                exit_code,
            } => {
                info!(
                    "Container died: {} (exit code: {:?})",
                    container_id, exit_code
                );

                // Remove from warm pool since container is dead
                if let Err(e) = self.warm_pool.remove_container_by_id(&container_id).await {
                    warn!(
                        "Failed to remove dead container {} from warm pool: {}",
                        container_id, e
                    );
                }
            }

            ContainerEvent::Stop { container_id } => {
                info!("Container stopped: {}", container_id);

                // Mark as stopped in warm pool
                if !self
                    .warm_pool
                    .set_state_by_container_id(&container_id, InstanceState::Stopped)
                    .await
                {
                    warn!("Failed to mark container {} as stopped", container_id);
                }
            }

            ContainerEvent::Kill { container_id } => {
                info!("Container killed: {}", container_id);

                // Remove from warm pool since container was killed
                if let Err(e) = self.warm_pool.remove_container_by_id(&container_id).await {
                    warn!(
                        "Failed to remove killed container {} from warm pool: {}",
                        container_id, e
                    );
                }
            }

            ContainerEvent::Remove { container_id } => {
                info!("Container removed: {}", container_id);

                // Remove from warm pool since container no longer exists
                if let Err(e) = self.warm_pool.remove_container_by_id(&container_id).await {
                    warn!(
                        "Failed to remove deleted container {} from warm pool: {}",
                        container_id, e
                    );
                }
            }

            ContainerEvent::Start { container_id } => {
                info!("Container started: {}", container_id);

                // Mark as warm idle when container starts
                if !self
                    .warm_pool
                    .set_state_by_container_id(&container_id, InstanceState::WarmIdle)
                    .await
                {
                    warn!("Failed to mark container {} as warm idle", container_id);
                }
            }

            ContainerEvent::Create { container_id } => {
                info!("Container created: {}", container_id);
                // No action needed for create events
            }
        }

        Ok(())
    }

    async fn sync_container_states(
        warm_pool: &WarmPool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Get all containers in warm pool
        let all_containers = warm_pool.list_all_containers().await;

        for (_fn_key, containers) in all_containers {
            for container in containers {
                // Check if container still exists in Docker
                // This would require access to the invoker, so we'll implement this
                // as a separate method that can be called with invoker reference
                info!("Syncing container state for: {}", container.container_id);
            }
        }

        Ok(())
    }
}
