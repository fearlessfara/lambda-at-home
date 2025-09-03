use bollard::Docker;
use bollard::container::{
    Config, CreateContainerOptions, StartContainerOptions, StopContainerOptions,
    RemoveContainerOptions, LogsOptions, LogOutput,
};

use bollard::models::{ContainerCreateResponse, HostConfig, 
                     RestartPolicy, RestartPolicyNameEnum};
use lambda_models::{Function, Config as AppConfig, LambdaError};
use std::collections::HashMap;
use futures_util::StreamExt;
use tracing::{info, error, instrument};
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct CreateSpec {
    pub image: String,
    pub name: String,
    pub env: Vec<(String, String)>,
    pub extra_hosts: Vec<String>,
    pub read_only_root_fs: bool,
    pub user: Option<String>,
    pub cap_drop: Vec<String>,
    pub no_new_privileges: bool,
    pub mounts: Vec<(String, String, bool)>, // (src,dst,ro)
    pub ulimits: Vec<(String, i64)>,         // (name, soft/hard same)
    pub labels: Vec<(String, String)>,
    pub network: Option<String>,
}

impl Default for CreateSpec {
    fn default() -> Self {
        Self {
            image: "test:latest".to_string(),
            name: "test-container".to_string(),
            env: vec![],
            extra_hosts: vec![],
            read_only_root_fs: false,
            user: None,
            cap_drop: vec![],
            no_new_privileges: false,
            mounts: vec![],
            ulimits: vec![],
            labels: vec![],
            network: None,
        }
    }
}

#[async_trait]
pub trait DockerLike: Send + Sync + 'static {
    async fn create(&self, spec: CreateSpec) -> anyhow::Result<String>; // returns container_id
    async fn start(&self, container_id: &str) -> anyhow::Result<()>;
    async fn stop(&self, container_id: &str, timeout_secs: u64) -> anyhow::Result<()>;
    async fn remove(&self, container_id: &str, force: bool) -> anyhow::Result<()>;
    async fn inspect_running(&self, container_id: &str) -> anyhow::Result<bool>;
}

pub struct Invoker {
    docker: Docker,
    config: AppConfig,
}

impl Invoker {
    pub async fn new(config: AppConfig) -> Result<Self, LambdaError> {
        let docker = Docker::connect_with_socket_defaults()
            .map_err(|e| LambdaError::DockerError { message: e.to_string() })?;
        
        Ok(Self { docker, config })
    }

    #[instrument(skip(self))]
    pub async fn create_container(&self, function: &Function, image_ref: &str, env_vars: HashMap<String, String>) -> Result<String, LambdaError> {
        let container_name = format!("lambda-{}-{}", function.function_name, uuid::Uuid::new_v4());
        
        // Build environment variables
        let mut env = Vec::new();
        env.push("AWS_LAMBDA_RUNTIME_API=host.docker.internal:9001".to_string());
        env.push("AWS_LAMBDA_FUNCTION_NAME=".to_string() + &function.function_name);
        env.push("AWS_LAMBDA_FUNCTION_VERSION=".to_string() + &function.version);
        env.push("AWS_LAMBDA_FUNCTION_MEMORY_SIZE=".to_string() + &function.memory_size.to_string());
        env.push("AWS_LAMBDA_LOG_GROUP_NAME=/aws/lambda/".to_string() + &function.function_name);
        env.push("AWS_LAMBDA_LOG_STREAM_NAME=".to_string() + &uuid::Uuid::new_v4().to_string());
        env.push("AWS_LAMBDA_RUNTIME_DIR=/var/runtime".to_string());
        env.push("LAMBDA_TASK_ROOT=/var/task".to_string());
        env.push("LAMBDA_RUNTIME_DIR=/var/runtime".to_string());
        env.push("TZ=UTC".to_string());
        
        // Add custom environment variables
        for (key, value) in env_vars {
            env.push(format!("{}={}", key, value));
        }
        
        // Security configuration
        let host_config = HostConfig {
            memory: Some((function.memory_size * 1024 * 1024) as i64), // Convert MB to bytes
            memory_swap: Some(-1), // Disable swap
            cpu_quota: Some(100000), // 1 CPU core
            cpu_period: Some(100000),
            pids_limit: Some(1024),
            readonly_rootfs: Some(true),
            tmpfs: Some(HashMap::from([
                ("/tmp".to_string(), format!("size={}m", self.config.defaults.tmp_mb))
            ])),
            restart_policy: Some(RestartPolicy {
                name: Some(RestartPolicyNameEnum::NO),
                maximum_retry_count: None,
            }),
            cap_drop: Some(vec!["ALL".to_string()]),
            cap_add: None,
            security_opt: Some(vec!["no-new-privileges:true".to_string()]),
            // Add host mapping for Runtime API connectivity
            extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
            ..Default::default()
        };
        
        let container_config = Config {
            image: Some(image_ref.to_string()),
            env: Some(env),
            host_config: Some(host_config),
            working_dir: Some("/var/task".to_string()),
            user: Some("1000:1000".to_string()), // Non-root user
            ..Default::default()
        };
        
        let options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };
        
        let response: ContainerCreateResponse = self.docker
            .create_container(Some(options), container_config)
            .await
            .map_err(|e| LambdaError::DockerError { message: e.to_string() })?;
        
        info!("Created container: {} with ID: {}", container_name, response.id);
        Ok(response.id)
    }

    #[instrument(skip(self))]
    pub async fn start_container(&self, container_id: &str) -> Result<(), LambdaError> {
        let options = StartContainerOptions::<String> {
            ..Default::default()
        };
        
        self.docker
            .start_container(container_id, Some(options))
            .await
            .map_err(|e| LambdaError::DockerError { message: e.to_string() })?;
        
        info!("Started container: {}", container_id);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn stop_container(&self, container_id: &str) -> Result<(), LambdaError> {
        let options = StopContainerOptions {
            t: 10, // 10 second grace period
        };
        
        self.docker
            .stop_container(container_id, Some(options))
            .await
            .map_err(|e| LambdaError::DockerError { message: e.to_string() })?;
        
        info!("Stopped container: {}", container_id);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn remove_container(&self, container_id: &str) -> Result<(), LambdaError> {
        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };
        
        self.docker
            .remove_container(container_id, Some(options))
            .await
            .map_err(|e| LambdaError::DockerError { message: e.to_string() })?;
        
        info!("Removed container: {}", container_id);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_container_logs(&self, container_id: &str) -> Result<String, LambdaError> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            timestamps: true,
            ..Default::default()
        };
        
        let mut stream = self.docker
            .logs(container_id, Some(options));
        
        let mut logs = String::new();
        while let Some(log) = stream.next().await {
            match log {
                Ok(LogOutput::StdOut { message }) => {
                    logs.push_str(&String::from_utf8_lossy(&message));
                }
                Ok(LogOutput::StdErr { message }) => {
                    logs.push_str(&String::from_utf8_lossy(&message));
                }
                Ok(LogOutput::StdIn { message }) => {
                    logs.push_str(&String::from_utf8_lossy(&message));
                }
                Ok(LogOutput::Console { message }) => {
                    logs.push_str(&String::from_utf8_lossy(&message));
                }
                Err(e) => {
                    error!("Error reading container logs: {}", e);
                    break;
                }
            }
        }
        
        Ok(logs)
    }

    #[instrument(skip(self))]
    pub async fn wait_for_container(&self, container_id: &str, timeout_ms: u64) -> Result<i64, LambdaError> {
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let start = std::time::Instant::now();
        
        loop {
            if start.elapsed() >= timeout {
                return Err(LambdaError::ContainerTimeout { timeout_ms });
            }
            
            // Check if container is still running
            match self.docker.inspect_container(container_id, None).await {
                Ok(container) => {
                    if let Some(state) = container.state {
                        if let Some(status) = state.status {
                            if status.to_string() == "exited" {
                                if let Some(exit_code) = state.exit_code {
                                    return Ok(exit_code);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error inspecting container {}: {}", container_id, e);
                    return Err(LambdaError::DockerError { message: e.to_string() });
                }
            }
            
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}
