use anyhow::Result;
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};
use uuid::Uuid;

use crate::container_lifecycle::ContainerLifecycleManager;
use crate::docker::DockerManager;
use crate::lambda_runtime_api::LambdaRuntimeService;
use crate::models::*;

#[derive(Clone)]
pub struct LambdaExecutor {
    docker_manager: DockerManager,
    lifecycle_manager: std::sync::Arc<ContainerLifecycleManager>,
    lambda_service: std::sync::Arc<LambdaRuntimeService>,
}

impl LambdaExecutor {
    pub fn new(
        docker_manager: DockerManager,
        lifecycle_manager: std::sync::Arc<ContainerLifecycleManager>,
        lambda_service: std::sync::Arc<LambdaRuntimeService>,
    ) -> Self {
        Self {
            docker_manager,
            lifecycle_manager,
            lambda_service,
        }
    }

    pub async fn invoke_function(
        &self,
        function_id: &Uuid,
        request: &RICInvokeRequest,
    ) -> Result<InvokeFunctionResponse> {
        let start_time = Instant::now();

        // Get container from lifecycle manager
        let container_info = self.lifecycle_manager.get_or_create_container(function_id).await?;
        
        // Register container with Lambda service
        self.lambda_service.register_container(container_info.id.clone(), *function_id).await;

        // Queue the invocation in the Lambda service
        let request_id = self.lambda_service.queue_invocation(*function_id, request.payload.clone()).await?;

        // Wait for the RIC to poll and process the invocation
        let response = self.wait_for_response(&request_id, Duration::from_secs(30)).await?;

        // Mark container as available after invocation completes
        if let Err(e) = self.lifecycle_manager.mark_container_available(function_id, &container_info.id).await {
            warn!("Failed to mark container {} as available: {}", container_info.id, e);
        }

        Ok(InvokeFunctionResponse {
            status_code: 200, // Lambda always returns 200 unless there's an error
            function_error: None,
            logs: vec![], // TODO: Collect logs from RIC
            payload: Some(response),
            duration_ms: Some(start_time.elapsed().as_millis() as u64),
            request_id,
        })
    }

    /// Wait for the RIC to process the invocation and return the response
    async fn wait_for_response(&self, request_id: &str, timeout: Duration) -> Result<Value> {
        let start_time = Instant::now();
        
        while start_time.elapsed() < timeout {
            // Check for completed response
            if let Some(response) = self.lambda_service.get_response(request_id).await? {
                info!("Received response for invocation {}", request_id);
                return Ok(response.response);
            }

            // Check for error response
            if let Some(error) = self.lambda_service.get_error(request_id).await? {
                warn!("Received error for invocation {}: {} - {}", request_id, error.error_type, error.error_message);
                return Err(anyhow::anyhow!("Function error: {} - {}", error.error_type, error.error_message));
            }

            // Wait a bit before checking again
            sleep(Duration::from_millis(100)).await;
        }

        Err(anyhow::anyhow!("Timeout waiting for response from invocation {}", request_id))
    }

    pub async fn terminate_containers(&self, function_id: &Uuid) -> Result<()> {
        let containers = self.docker_manager.list_containers().await?;
        for container in containers {
            if let Some(labels) = container.labels {
                if labels.get("function_id").map(|id| id == &function_id.to_string()).unwrap_or(false) {
                    if let Some(id) = container.id {
                        self.docker_manager.stop_container(&id).await?;
                        self.docker_manager.remove_container(&id).await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn get_container_stats(&self) -> Result<ContainerStats> {
        let mut stats = ContainerStats::default();
        let containers = self.docker_manager.list_containers().await?;

        for container in containers {
            if let Some(id) = container.id {
                let status = container.state.unwrap_or_default();
                let labels = container.labels.unwrap_or_default();
                let _function_id = labels.get("function_id").cloned().unwrap_or_default();

                if status == "running" {
                    stats.active_containers += 1;
                } else {
                    stats.idle_containers += 1;
                }
                stats.total_containers += 1;
                stats.total_invocations += 1;

                stats.containers.push(ContainerInfo {
                    id,
                    status,
                    invocation_count: 1, // TODO: Track actual invocation count
                    last_used_at: chrono::Utc::now(),
                    created_at: chrono::Utc::now(), // TODO: Get actual creation time
                });
            }
        }

        Ok(stats)
    }
}
