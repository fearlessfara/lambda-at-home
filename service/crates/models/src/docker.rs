use serde::{Deserialize, Serialize};

/// Docker system information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerSystemInfo {
    pub containers: i64,
    pub containers_running: i64,
    pub containers_paused: i64,
    pub containers_stopped: i64,
    pub images: i64,
    pub driver: String,
    pub memory_total: u64,
    pub memory_available: u64,
    pub cpu_count: i64,
    pub kernel_version: String,
    pub operating_system: String,
    pub architecture: String,
    pub docker_root_dir: String,
    pub storage_driver: String,
    pub logging_driver: String,
    pub cgroup_driver: String,
    pub cgroup_version: String,
    pub n_events_listener: i64,
    pub n_goroutines: i64,
    pub system_time: String,
    pub server_version: String,
}

/// Docker disk usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerDiskUsage {
    pub layers_size: u64,
    pub images: Vec<DockerImageUsage>,
    pub containers: Vec<DockerContainerUsage>,
    pub volumes: Vec<DockerVolumeUsage>,
    pub build_cache: Vec<DockerBuildCacheUsage>,
}

/// Docker image usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerImageUsage {
    pub id: String,
    pub parent_id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    pub created: i64,
    pub shared_size: u64,
    pub size: u64,
    pub virtual_size: u64,
    pub labels: std::collections::HashMap<String, String>,
    pub containers: i64,
}

/// Docker container usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainerUsage {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub image_id: String,
    pub command: String,
    pub created: i64,
    pub size_rw: u64,
    pub size_root_fs: u64,
    pub labels: std::collections::HashMap<String, String>,
    pub state: String,
    pub status: String,
}

/// Docker volume usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerVolumeUsage {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: String,
    pub size: u64,
    pub labels: std::collections::HashMap<String, String>,
    pub scope: String,
    pub options: std::collections::HashMap<String, String>,
}

/// Docker build cache usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerBuildCacheUsage {
    pub id: String,
    pub parent: String,
    pub r#type: String,
    pub description: String,
    pub in_use: bool,
    pub shared: bool,
    pub size: u64,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub usage_count: i64,
}

/// Docker version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerVersion {
    pub version: String,
    pub api_version: String,
    pub min_api_version: String,
    pub git_commit: String,
    pub go_version: String,
    pub os: String,
    pub arch: String,
    pub kernel_version: String,
    pub experimental: bool,
    pub build_time: String,
}

/// Comprehensive Docker statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerStats {
    pub system_info: DockerSystemInfo,
    pub disk_usage: DockerDiskUsage,
    pub version: DockerVersion,
    pub cache_stats: Option<CacheStats>,
}

/// Cache statistics (from our function cache)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub functions: CacheTypeStats,
    pub concurrency: CacheTypeStats,
    pub env_vars: CacheTypeStats,
    pub secrets: CacheTypeStats,
}

/// Cache statistics for a specific type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTypeStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub invalidations: u64,
    pub hit_rate: f64,
    pub size: usize,
}

/// Lambda service statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LambdaServiceStats {
    pub total_functions: u64,
    pub active_functions: u64,
    pub stopped_functions: u64,
    pub failed_functions: u64,
    pub total_memory_mb: u64,
    pub total_cpu_cores: u64,
    pub warm_containers: u64,
    pub active_containers: u64,
    pub idle_containers: u64,
    pub total_invocations_24h: u64,
    pub successful_invocations_24h: u64,
    pub failed_invocations_24h: u64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: f64,
    pub min_duration_ms: f64,
}
