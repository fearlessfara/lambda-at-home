use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server_address: String,
    pub port: u16,
    pub storage_path: String,
    pub docker_config: DockerConfig,
    pub execution_config: ExecutionConfig,
    pub ric_config: RICConfig,
    pub lifecycle_config: LifecycleConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub network_name: String,
    pub container_prefix: String,
    pub container_labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub max_concurrent_executions: usize,
    pub max_memory_mb: u64,
    pub max_cpu_shares: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RICConfig {
    pub port: u16,
    pub startup_timeout_seconds: u64,
    pub shutdown_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    pub max_idle_time_seconds: u64,
    pub max_container_age_seconds: u64,
    pub cleanup_interval_seconds: u64,
    pub max_containers_per_function: usize,
    pub max_global_containers: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_address: "0.0.0.0".to_string(),
            port: 8080,
            storage_path: "/tmp/lambda-functions".to_string(),
            docker_config: DockerConfig {
                network_name: "lambda-network".to_string(),
                container_prefix: "lambda-".to_string(),
                container_labels: HashMap::new(),
            },
            execution_config: ExecutionConfig {
                max_concurrent_executions: 10,
                max_memory_mb: 256,
                max_cpu_shares: 1.0,
            },
            ric_config: RICConfig {
                port: 8080,
                startup_timeout_seconds: 30,
                shutdown_timeout_seconds: 10,
            },
            lifecycle_config: LifecycleConfig {
                max_idle_time_seconds: 300, // 5 minutes
                max_container_age_seconds: 3600, // 1 hour
                cleanup_interval_seconds: 60, // 1 minute
                max_containers_per_function: 3,
                max_global_containers: 50,
            },
        }
    }
}

impl Config {
    pub fn load(config_path: &str) -> Result<Self> {
        let config_str = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}