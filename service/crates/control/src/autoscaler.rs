use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, instrument, warn};

use crate::{registry::ControlPlane, warm_pool::InstanceState};

pub struct Autoscaler {
    control: Arc<ControlPlane>,
}

impl Autoscaler {
    pub fn new(control: Arc<ControlPlane>) -> Self {
        Self { control }
    }

    #[instrument(skip(self))]
    pub async fn start(self) {
        let tick = Duration::from_millis(250);
        loop {
            if let Err(e) = self.reconcile_once().await {
                warn!("autoscaler reconcile error: {}", e);
            }
            tokio::time::sleep(tick).await;
        }
    }

    async fn reconcile_once(&self) -> anyhow::Result<()> {
        let queues = self.control.queues();
        let sizes = queues.snapshot_sizes();
        for (key, qsize) in sizes {
            if qsize == 0 {
                continue;
            }
            let idle = self
                .control
                .warm_pool()
                .count_state(&key, InstanceState::WarmIdle)
                .await;
            let stopped = self
                .control
                .warm_pool()
                .count_state(&key, InstanceState::Stopped)
                .await;
            let (to_restart, to_create) = plan_scale(qsize, idle, stopped);
            if to_restart == 0 && to_create == 0 {
                continue;
            }

            // Restart stopped ones first
            let stopped_ids = self.control.warm_pool().list_stopped(&key).await;
            for cid in stopped_ids.into_iter().take(to_restart) {
                info!(
                    "autoscaler: restarting stopped container {} for {}",
                    cid, key.function_name
                );
                if let Err(e) = self.control.invoker().start_container(&cid).await {
                    error!("start failed: {}", e);
                    continue;
                }
                let _ = self
                    .control
                    .warm_pool()
                    .set_state_by_container_id(&cid, InstanceState::WarmIdle)
                    .await;
            }

            // Create new containers as needed
            for _ in 0..to_create {
                if let Err(e) = self.create_one(&key).await {
                    error!("create failed: {}", e);
                }
            }
        }
        Ok(())
    }

    async fn create_one(&self, key: &crate::queues::FnKey) -> anyhow::Result<()> {
        let function = self.control.get_function(&key.function_name).await?;
        let image_ref = format!(
            "lambda-home/{}:{}",
            function.function_name, function.code_sha256
        );
        let mut packaging = lambda_packaging::PackagingService::new(self.control.config());
        packaging.build_image(&function, &image_ref).await?;

        let instance_id = uuid::Uuid::new_v4().to_string();
        let mut env_vars = function.environment.clone();
        env_vars.insert("LAMBDAH_INSTANCE_ID".to_string(), instance_id.clone());
        let container_id = self
            .control
            .invoker()
            .create_container(&function, &image_ref, env_vars)
            .await?;
        self.control
            .invoker()
            .start_container(&container_id)
            .await?;

        let wc = crate::warm_pool::WarmContainer {
            container_id: container_id.clone(),
            instance_id: instance_id.clone(),
            function_id: function.function_id,
            image_ref: image_ref.clone(),
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
            state: InstanceState::WarmIdle,
        };
        self.control
            .warm_pool()
            .add_warm_container(key.clone(), wc)
            .await;
        info!(
            "autoscaler: created container {} for {}",
            container_id, key.function_name
        );
        Ok(())
    }
}

// Pure decision function to plan scaling from current queue depth/state.
// Returns (restart_count, create_count).
pub fn plan_scale(queue_size: usize, warm_idle: usize, stopped: usize) -> (usize, usize) {
    if queue_size <= warm_idle {
        return (0, 0);
    }
    let need = queue_size - warm_idle;
    let restart = need.min(stopped);
    let create = need - restart;
    (restart, create)
}

