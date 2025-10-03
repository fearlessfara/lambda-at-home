use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::image::{ListImagesOptions, RemoveImageOptions};
use bollard::models::EventMessage;
use bollard::Docker;
// Unused imports removed - these types are re-exported by bollard::models

use async_trait::async_trait;
use bollard::models::{ContainerCreateResponse, HostConfig, RestartPolicy, RestartPolicyNameEnum};
use futures_util::StreamExt;
use lambda_models::{
    Config as AppConfig, DockerDiskUsage, DockerStats, DockerSystemInfo,
    DockerVersion as LambdaDockerVersion, Function, LambdaError,
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{error, info, instrument, warn};

#[derive(Clone, Debug)]
pub enum ContainerEvent {
    Die {
        container_id: String,
        exit_code: Option<i64>,
    },
    Stop {
        container_id: String,
    },
    Kill {
        container_id: String,
    },
    Remove {
        container_id: String,
    },
    Start {
        container_id: String,
    },
    Create {
        container_id: String,
    },
}

pub type ContainerEventSender = mpsc::UnboundedSender<ContainerEvent>;

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
    async fn get_docker_stats(&self) -> anyhow::Result<DockerStats>;
    async fn remove_image(&self, image_ref: &str, force: bool) -> anyhow::Result<()>;
    async fn list_lambda_images(&self) -> anyhow::Result<Vec<String>>;
}

pub struct Invoker {
    docker: Docker,
    config: AppConfig,
    event_sender: Option<ContainerEventSender>,
}

impl Invoker {
    pub async fn new(config: AppConfig) -> Result<Self, LambdaError> {
        let docker = if let Ok(docker_host) = std::env::var("DOCKER_HOST") {
            // Use DOCKER_HOST environment variable if available (for CI/Docker-in-Docker)
            if docker_host.starts_with("tcp://") {
                Docker::connect_with_http(&docker_host, 120, bollard::API_DEFAULT_VERSION)
                    .map_err(|e| LambdaError::DockerError {
                        message: format!("Failed to connect to Docker at {docker_host}: {e}"),
                    })?
            } else {
                // Fallback to socket connection
                Docker::connect_with_socket_defaults().map_err(|e| LambdaError::DockerError {
                    message: e.to_string(),
                })?
            }
        } else {
            // Default to Unix socket connection
            Docker::connect_with_socket_defaults().map_err(|e| LambdaError::DockerError {
                message: e.to_string(),
            })?
        };

        Ok(Self {
            docker,
            config,
            event_sender: None,
        })
    }

    pub fn with_event_sender(mut self, sender: ContainerEventSender) -> Self {
        self.event_sender = Some(sender);
        self
    }

    #[instrument(skip(self))]
    pub async fn start_events_monitor(&self) -> Result<(), LambdaError> {
        let docker = self.docker.clone();
        let event_sender = self.event_sender.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::monitor_docker_events(docker, event_sender).await {
                error!("Docker events monitor failed: {}", e);
            }
        });

        info!("Started Docker events monitor");
        Ok(())
    }

    #[instrument(skip(self, event_sender))]
    pub async fn start_events_monitor_with_sender(
        &self,
        event_sender: ContainerEventSender,
    ) -> Result<(), LambdaError> {
        let docker = self.docker.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::monitor_docker_events(docker, Some(event_sender)).await {
                error!("Docker events monitor failed: {}", e);
            }
        });

        info!("Started Docker events monitor with sender");
        Ok(())
    }

    async fn monitor_docker_events(
        docker: Docker,
        event_sender: Option<ContainerEventSender>,
    ) -> Result<(), LambdaError> {
        let mut events_stream = docker.events::<String>(None);

        info!("Docker events monitor started");

        while let Some(event_result) = events_stream.next().await {
            match event_result {
                Ok(event) => {
                    if let Some(sender) = &event_sender {
                        if let Some(container_event) = Self::parse_docker_event(event) {
                            if let Err(e) = sender.send(container_event) {
                                warn!("Failed to send container event: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving Docker event: {}", e);
                    // Continue monitoring even if we get errors
                }
            }
        }

        warn!("Docker events stream ended");
        Ok(())
    }

    fn parse_docker_event(event: EventMessage) -> Option<ContainerEvent> {
        let actor = event.actor?;
        let container_id = actor.id?;

        match event.action.as_deref() {
            Some("die") => {
                let exit_code = actor
                    .attributes
                    .and_then(|attrs| attrs.get("exitCode").cloned())
                    .and_then(|code| code.parse::<i64>().ok());

                Some(ContainerEvent::Die {
                    container_id,
                    exit_code,
                })
            }
            Some("stop") => Some(ContainerEvent::Stop { container_id }),
            Some("kill") => Some(ContainerEvent::Kill { container_id }),
            Some("remove") => Some(ContainerEvent::Remove { container_id }),
            Some("start") => Some(ContainerEvent::Start { container_id }),
            Some("create") => Some(ContainerEvent::Create { container_id }),
            _ => None, // Ignore other events
        }
    }

    #[instrument(skip(self))]
    pub async fn create_container(
        &self,
        function: &Function,
        image_ref: &str,
        env_vars: HashMap<String, String>,
    ) -> Result<String, LambdaError> {
        let container_name = format!("lambda-{}-{}", function.function_name, uuid::Uuid::new_v4());

        // Build environment variables
        let runtime_api = format!("host.docker.internal:{}", self.config.server.port_runtime_api);
        let mut env = vec![
            format!("AWS_LAMBDA_RUNTIME_API={}", runtime_api),
            "AWS_LAMBDA_FUNCTION_NAME=".to_string() + &function.function_name,
            "AWS_LAMBDA_FUNCTION_VERSION=".to_string() + &function.version,
            "AWS_LAMBDA_FUNCTION_MEMORY_SIZE=".to_string() + &function.memory_size.to_string(),
            "AWS_LAMBDA_LOG_GROUP_NAME=/aws/lambda/".to_string() + &function.function_name,
            "AWS_LAMBDA_LOG_STREAM_NAME=".to_string() + &uuid::Uuid::new_v4().to_string(),
            "AWS_LAMBDA_RUNTIME_DIR=/var/runtime".to_string(),
            "LAMBDA_TASK_ROOT=/var/task".to_string(),
            "LAMBDA_RUNTIME_DIR=/var/runtime".to_string(),
            "TZ=UTC".to_string(),
        ];

        // Add custom environment variables
        for (key, value) in env_vars {
            env.push(format!("{key}={value}"));
        }

        // Security configuration
        let host_config = HostConfig {
            memory: Some((function.memory_size * 1024 * 1024) as i64), // Convert MB to bytes
            memory_swap: Some(-1),                                     // Disable swap
            cpu_quota: Some(100000),                                   // 1 CPU core
            cpu_period: Some(100000),
            pids_limit: Some(1024),
            readonly_rootfs: Some(true),
            tmpfs: Some(HashMap::from([(
                "/tmp".to_string(),
                format!("size={}m", self.config.defaults.tmp_mb),
            )])),
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

        let response: ContainerCreateResponse = self
            .docker
            .create_container(Some(options), container_config)
            .await
            .map_err(|e| LambdaError::DockerError {
                message: e.to_string(),
            })?;

        info!(
            "Created container: {} with ID: {}",
            container_name, response.id
        );
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
            .map_err(|e| LambdaError::DockerError {
                message: e.to_string(),
            })?;

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
            .map_err(|e| LambdaError::DockerError {
                message: e.to_string(),
            })?;

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
            .map_err(|e| LambdaError::DockerError {
                message: e.to_string(),
            })?;

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

        let mut stream = self.docker.logs(container_id, Some(options));

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
    pub async fn wait_for_container(
        &self,
        container_id: &str,
        timeout_ms: u64,
    ) -> Result<i64, LambdaError> {
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
                    return Err(LambdaError::DockerError {
                        message: e.to_string(),
                    });
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    #[instrument(skip(self))]
    pub async fn get_docker_stats(&self) -> anyhow::Result<DockerStats> {
        // Get system info
        let system_info = self
            .docker
            .info()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get Docker system info: {}", e))?;

        // Get disk usage
        let disk_usage = self
            .docker
            .df()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get Docker disk usage: {}", e))?;

        // Get version
        let version = self
            .docker
            .version()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get Docker version: {}", e))?;

        // Convert to our models
        let docker_system_info = DockerSystemInfo {
            containers: system_info.containers.unwrap_or(0),
            containers_running: system_info.containers_running.unwrap_or(0),
            containers_paused: system_info.containers_paused.unwrap_or(0),
            containers_stopped: system_info.containers_stopped.unwrap_or(0),
            images: system_info.images.unwrap_or(0),
            driver: system_info.driver.unwrap_or_default(),
            memory_total: system_info.mem_total.unwrap_or(0) as u64,
            memory_available: 0, // Not available in SystemInfo
            cpu_count: system_info.ncpu.unwrap_or(0),
            kernel_version: system_info.kernel_version.unwrap_or_default(),
            operating_system: system_info.operating_system.unwrap_or_default(),
            architecture: system_info.architecture.unwrap_or_default(),
            docker_root_dir: system_info.docker_root_dir.unwrap_or_default(),
            storage_driver: "unknown".to_string(), // Not available in SystemInfo
            logging_driver: system_info.logging_driver.unwrap_or_default(),
            cgroup_driver: system_info
                .cgroup_driver
                .map(|e| e.to_string())
                .unwrap_or_default(),
            cgroup_version: system_info
                .cgroup_version
                .map(|e| e.to_string())
                .unwrap_or_default(),
            n_events_listener: system_info.n_events_listener.unwrap_or(0),
            n_goroutines: system_info.n_goroutines.unwrap_or(0),
            system_time: system_info.system_time.unwrap_or_default(),
            server_version: system_info.server_version.unwrap_or_default(),
        };

        let docker_disk_usage = DockerDiskUsage {
            layers_size: disk_usage.layers_size.unwrap_or(0) as u64,
            images: disk_usage
                .images
                .unwrap_or_default()
                .into_iter()
                .map(|img| lambda_models::DockerImageUsage {
                    id: img.id,
                    parent_id: img.parent_id,
                    repo_tags: img.repo_tags,
                    repo_digests: img.repo_digests,
                    created: img.created,
                    shared_size: img.shared_size as u64,
                    size: img.size as u64,
                    virtual_size: img.virtual_size.unwrap_or(0) as u64,
                    labels: img.labels,
                    containers: img.containers,
                })
                .collect(),
            containers: disk_usage
                .containers
                .unwrap_or_default()
                .into_iter()
                .map(|cont| lambda_models::DockerContainerUsage {
                    id: cont.id.unwrap_or_default(),
                    names: cont.names.unwrap_or_default(),
                    image: cont.image.unwrap_or_default(),
                    image_id: cont.image_id.unwrap_or_default(),
                    command: cont.command.unwrap_or_default(),
                    created: cont.created.unwrap_or(0),
                    size_rw: cont.size_rw.unwrap_or(0) as u64,
                    size_root_fs: cont.size_root_fs.unwrap_or(0) as u64,
                    labels: cont.labels.unwrap_or_default(),
                    state: cont.state.unwrap_or_default(),
                    status: cont.status.unwrap_or_default(),
                })
                .collect(),
            volumes: disk_usage
                .volumes
                .unwrap_or_default()
                .into_iter()
                .map(|vol| {
                    lambda_models::DockerVolumeUsage {
                        name: vol.name,
                        driver: vol.driver,
                        mountpoint: vol.mountpoint,
                        created_at: vol.created_at.unwrap_or_default(),
                        size: 0, // Not available in Volume
                        labels: vol.labels,
                        scope: vol.scope.map(|e| e.to_string()).unwrap_or_default(),
                        options: vol.options,
                    }
                })
                .collect(),
            build_cache: disk_usage
                .build_cache
                .unwrap_or_default()
                .into_iter()
                .map(|cache| lambda_models::DockerBuildCacheUsage {
                    id: cache.id.unwrap_or_default(),
                    parent: cache.parent.unwrap_or_default(),
                    r#type: cache.typ.map(|e| e.to_string()).unwrap_or_default(),
                    description: cache.description.unwrap_or_default(),
                    in_use: cache.in_use.unwrap_or(false),
                    shared: cache.shared.unwrap_or(false),
                    size: cache.size.unwrap_or(0) as u64,
                    created_at: cache.created_at.unwrap_or_default(),
                    last_used_at: cache.last_used_at,
                    usage_count: cache.usage_count.unwrap_or(0),
                })
                .collect(),
        };

        let docker_version = LambdaDockerVersion {
            version: version.version.unwrap_or_default(),
            api_version: version.api_version.unwrap_or_default(),
            min_api_version: version.min_api_version.unwrap_or_default(),
            git_commit: version.git_commit.unwrap_or_default(),
            go_version: version.go_version.unwrap_or_default(),
            os: version.os.unwrap_or_default(),
            arch: version.arch.unwrap_or_default(),
            kernel_version: version.kernel_version.unwrap_or_default(),
            experimental: version.experimental.map(|e| e == "true").unwrap_or(false),
            build_time: version.build_time.unwrap_or_default(),
        };

        Ok(DockerStats {
            system_info: docker_system_info,
            disk_usage: docker_disk_usage,
            version: docker_version,
            cache_stats: None, // Will be filled by the caller
        })
    }

    #[instrument(skip(self))]
    pub async fn remove_image(&self, image_ref: &str, force: bool) -> Result<(), LambdaError> {
        let options = RemoveImageOptions {
            force,
            noprune: false,
        };

        self.docker
            .remove_image(image_ref, Some(options), None)
            .await
            .map_err(|e| LambdaError::DockerError {
                message: format!("Failed to remove image {image_ref}: {e}"),
            })?;

        info!("Removed Docker image: {}", image_ref);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn list_lambda_images(&self) -> Result<Vec<String>, LambdaError> {
        let options = ListImagesOptions::<String> {
            all: true,
            ..Default::default()
        };

        let images =
            self.docker
                .list_images(Some(options))
                .await
                .map_err(|e| LambdaError::DockerError {
                    message: format!("Failed to list images: {e}"),
                })?;

        // Filter for Lambda@Home images (those with lambda-home/ prefix)
        let lambda_images: Vec<String> = images
            .into_iter()
            .filter_map(|image| {
                image
                    .repo_tags
                    .into_iter()
                    .find(|tag| tag.starts_with("lambda-home/"))
                    .map(|tag| tag.to_string())
            })
            .collect();

        Ok(lambda_images)
    }
}

#[async_trait]
impl DockerLike for Invoker {
    async fn create(&self, spec: CreateSpec) -> anyhow::Result<String> {
        let container_name = spec.name.clone();

        let config = Config {
            image: Some(spec.image.clone()),
            env: Some(spec.env.iter().map(|(k, v)| format!("{k}={v}")).collect()),
            host_config: Some(HostConfig {
                memory: Some(self.config.defaults.memory_mb as i64 * 1024 * 1024),
                cpu_quota: Some(100000),
                cpu_period: Some(100000),
                readonly_rootfs: Some(spec.read_only_root_fs),
                cap_drop: Some(spec.cap_drop.clone()),
                security_opt: if spec.no_new_privileges {
                    Some(vec!["no-new-privileges:true".to_string()])
                } else {
                    None
                },
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::NO),
                    maximum_retry_count: None,
                }),
                ..Default::default()
            }),
            labels: Some(spec.labels.iter().cloned().collect()),
            ..Default::default()
        };

        let create_options = CreateContainerOptions {
            name: container_name.clone(),
            ..Default::default()
        };

        let response = self
            .docker
            .create_container(Some(create_options), config)
            .await?;

        Ok(response.id)
    }

    async fn start(&self, container_id: &str) -> anyhow::Result<()> {
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await?;
        Ok(())
    }

    async fn stop(&self, container_id: &str, timeout_secs: u64) -> anyhow::Result<()> {
        let options = StopContainerOptions {
            t: timeout_secs as i64,
        };
        self.docker
            .stop_container(container_id, Some(options))
            .await?;
        Ok(())
    }

    async fn remove(&self, container_id: &str, force: bool) -> anyhow::Result<()> {
        let options = RemoveContainerOptions {
            force,
            ..Default::default()
        };
        self.docker
            .remove_container(container_id, Some(options))
            .await?;
        Ok(())
    }

    async fn inspect_running(&self, container_id: &str) -> anyhow::Result<bool> {
        let container = self.docker.inspect_container(container_id, None).await?;
        Ok(container
            .state
            .is_some_and(|state| state.running.unwrap_or(false)))
    }

    async fn get_docker_stats(&self) -> anyhow::Result<DockerStats> {
        self.get_docker_stats()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn remove_image(&self, image_ref: &str, force: bool) -> anyhow::Result<()> {
        self.remove_image(image_ref, force)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn list_lambda_images(&self) -> anyhow::Result<Vec<String>> {
        self.list_lambda_images()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
