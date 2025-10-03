use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    pub data: DataConfig,
    pub docker: DockerConfig,
    pub defaults: DefaultsConfig,
    pub idle: IdleConfig,
    pub limits: LimitsConfig,
    pub warmup: WarmupConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ServerConfig {
    pub bind: String,
    pub port_user_api: u16,
    pub port_runtime_api: u16,
    pub max_request_body_size_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DataConfig {
    pub dir: String,
    pub db_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DockerConfig {
    pub host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DefaultsConfig {
    pub memory_mb: u64,
    pub timeout_ms: u64,
    pub tmp_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IdleConfig {
    pub soft_ms: u64,
    pub hard_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LimitsConfig {
    pub max_global_concurrency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WarmupConfig {
    pub enabled: bool,
    pub timeout_ms: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                bind: "127.0.0.1".to_string(),
                port_user_api: 8000,
                port_runtime_api: 8001,
                max_request_body_size_mb: 50, // 50MB default limit
            },
            data: DataConfig {
                dir: "data".to_string(),
                db_url: "sqlite://data/lhome.db".to_string(),
            },
            docker: DockerConfig {
                host: "".to_string(),
            },
            defaults: DefaultsConfig {
                memory_mb: 512,
                timeout_ms: 3000,
                tmp_mb: 512,
            },
            idle: IdleConfig {
                soft_ms: 45000,
                hard_ms: 300000,
            },
            limits: LimitsConfig {
                max_global_concurrency: 256,
            },
            warmup: WarmupConfig {
                enabled: true,
                timeout_ms: 30000, // 30 seconds timeout for warm-up
            },
        }
    }
}
