use anyhow::{Context, Result};
use bollard::container::{Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions};
use bollard::models::{PortBinding, PortMap};
use bollard::Docker;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DockerManager {
    docker: Docker,
}

impl DockerManager {
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()
            .context("Failed to connect to Docker")?;

        Ok(Self {
            docker,
        })
    }



    pub async fn create_container(
        &self,
        function_id: &Uuid,
        container_name: &str,
        environment_variables: &HashMap<String, String>,
        memory_limit: u64,
        cpu_limit: f64,
    ) -> Result<String> {
        let mut labels = HashMap::new();
        labels.insert("function_id".to_string(), function_id.to_string());

        // Create port mapping for RIC server (8080 -> random host port)
        let mut port_bindings = PortMap::new();
        port_bindings.insert(
            "8080/tcp".to_string(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some("0".to_string()), // 0 means random port
            }]),
        );

        let host_config = bollard::service::HostConfig {
            memory: Some((memory_limit * 1024 * 1024) as i64), // Convert MB to bytes
            cpu_shares: Some((cpu_limit * 1024.0) as i64),
            port_bindings: Some(port_bindings),
            extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]), // Add host.docker.internal mapping
            ..Default::default()
        };

        let config = Config {
            image: Some(format!("lambda-function-{}", function_id)),
            env: Some(
                environment_variables
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v))
                    .collect(),
            ),
            labels: Some(labels),
            host_config: Some(host_config),
            exposed_ports: Some({
                let mut ports = HashMap::new();
                ports.insert("8080/tcp".to_string(), HashMap::new());
                ports
            }),
            ..Default::default()
        };

        let options = Some(CreateContainerOptions {
            name: container_name.to_string(),
            platform: None,
        });

        let container = self
            .docker
            .create_container(options, config)
            .await
            .context("Failed to create container")?;

        Ok(container.id)
    }

    pub async fn start_container(&self, container_id: &str) -> Result<()> {
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;

        Ok(())
    }

    pub async fn stop_container(&self, container_id: &str) -> Result<()> {
        let options = Some(StopContainerOptions {
            t: 10,
        });

        self.docker
            .stop_container(container_id, options)
            .await
            .context("Failed to stop container")?;

        Ok(())
    }

    pub async fn remove_container(&self, container_id: &str) -> Result<()> {
        let options = Some(RemoveContainerOptions {
            force: true,
            ..Default::default()
        });

        self.docker
            .remove_container(container_id, options)
            .await
            .context("Failed to remove container")?;

        Ok(())
    }

    pub async fn list_containers(&self) -> Result<Vec<bollard::models::ContainerSummary>> {
        let options = Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        });

        let containers = self
            .docker
            .list_containers(options)
            .await
            .context("Failed to list containers")?;

        Ok(containers)
    }

    pub async fn get_container_ip(&self, container_id: &str) -> Result<String> {
        let container = self
            .docker
            .inspect_container(container_id, None)
            .await
            .context("Failed to inspect container")?;

        let networks = container
            .network_settings
            .ok_or_else(|| anyhow::anyhow!("No network settings found"))?
            .networks
            .ok_or_else(|| anyhow::anyhow!("No networks found"))?;

        let network = networks
            .values()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No network found"))?;

        let ip_address = network
            .ip_address
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No IP address found"))?;

        Ok(ip_address.clone())
    }

    pub async fn get_container_mapped_port(&self, container_id: &str, container_port: u16) -> Result<u16> {
        let container = self
            .docker
            .inspect_container(container_id, None)
            .await
            .context("Failed to inspect container")?;

        let port_bindings = container
            .network_settings
            .ok_or_else(|| anyhow::anyhow!("No network settings found"))?
            .ports
            .ok_or_else(|| anyhow::anyhow!("No port bindings found"))?;

        let port_key = format!("{}/tcp", container_port);
        let bindings = port_bindings
            .get(&port_key)
            .ok_or_else(|| anyhow::anyhow!("No port binding found for {}", port_key))?;

        let binding = bindings
            .as_ref()
            .and_then(|b| b.first())
            .ok_or_else(|| anyhow::anyhow!("No port binding found for {}", port_key))?;

        let host_port = binding
            .host_port
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No host port found"))?
            .parse::<u16>()
            .context("Failed to parse host port")?;

        Ok(host_port)
    }

    /// Check if a container is running
    pub async fn is_container_running(&self, container_id: &str) -> Result<bool> {
        let container = self
            .docker
            .inspect_container(container_id, None)
            .await
            .context("Failed to inspect container")?;
        
        Ok(container.state.map(|s| s.running.unwrap_or(false)).unwrap_or(false))
    }
}

