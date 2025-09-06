use async_trait::async_trait;
use lambda_invoker::docker::{CreateSpec, DockerLike};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct FakeDocker {
    pub created: Arc<Mutex<Vec<CreateSpec>>>,
    pub started: Arc<Mutex<Vec<String>>>,
    pub stopped: Arc<Mutex<Vec<(String, u64)>>>,
    pub removed: Arc<Mutex<Vec<(String, bool)>>>,
    pub running: Arc<Mutex<bool>>,
    pub next_id: Arc<Mutex<u64>>,
}

impl FakeDocker {
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn last_created(&self) -> CreateSpec {
        self.created
            .lock()
            .await
            .last()
            .cloned()
            .expect("no create")
    }
}

#[async_trait]
impl DockerLike for FakeDocker {
    async fn create(&self, spec: CreateSpec) -> anyhow::Result<String> {
        self.created.lock().await.push(spec);
        let mut id = self.next_id.lock().await;
        *id += 1;
        Ok(format!("ctr-{}", *id))
    }
    async fn start(&self, container_id: &str) -> anyhow::Result<()> {
        self.started.lock().await.push(container_id.to_string());
        *self.running.lock().await = true;
        Ok(())
    }
    async fn stop(&self, container_id: &str, timeout_secs: u64) -> anyhow::Result<()> {
        self.stopped
            .lock()
            .await
            .push((container_id.to_string(), timeout_secs));
        *self.running.lock().await = false;
        Ok(())
    }
    async fn remove(&self, container_id: &str, force: bool) -> anyhow::Result<()> {
        self.removed
            .lock()
            .await
            .push((container_id.to_string(), force));
        Ok(())
    }
    async fn inspect_running(&self, _container_id: &str) -> anyhow::Result<bool> {
        Ok(*self.running.lock().await)
    }
    async fn get_docker_stats(&self) -> anyhow::Result<lambda_models::docker::DockerStats> {
        // Return mock Docker stats for testing
        Ok(lambda_models::docker::DockerStats {
            system_info: lambda_models::docker::DockerSystemInfo {
                containers: 0,
                containers_running: 0,
                containers_paused: 0,
                containers_stopped: 0,
                images: 0,
                driver: "overlay2".to_string(),
                memory_total: 8589934592,
                memory_available: 4294967296,
                cpu_count: 4,
                kernel_version: "5.4.0".to_string(),
                operating_system: "Docker Desktop".to_string(),
                architecture: "x86_64".to_string(),
                docker_root_dir: "/var/lib/docker".to_string(),
                storage_driver: "overlay2".to_string(),
                logging_driver: "json-file".to_string(),
                cgroup_driver: "systemd".to_string(),
                cgroup_version: "2".to_string(),
                n_events_listener: 0,
                n_goroutines: 0,
                system_time: "2023-01-01T00:00:00Z".to_string(),
                server_version: "20.10.0".to_string(),
            },
            disk_usage: lambda_models::docker::DockerDiskUsage {
                layers_size: 0,
                images: vec![],
                containers: vec![],
                volumes: vec![],
                build_cache: vec![],
            },
            version: lambda_models::docker::DockerVersion {
                version: "20.10.0".to_string(),
                api_version: "1.41".to_string(),
                min_api_version: "1.12".to_string(),
                git_commit: "test".to_string(),
                go_version: "go1.13.15".to_string(),
                os: "linux".to_string(),
                arch: "amd64".to_string(),
                kernel_version: "5.4.0".to_string(),
                experimental: false,
                build_time: "2023-01-01T00:00:00Z".to_string(),
            },
            cache_stats: None,
        })
    }
}

#[tokio::test]
async fn start_stop_remove_lifecycle() {
    let docker = FakeDocker::new();

    // Create & start
    let id = docker.create(Default::default()).await.unwrap();
    docker.start(&id).await.unwrap();
    assert!(docker.inspect_running(&id).await.unwrap());

    // Soft idle: stop
    docker.stop(&id, 2).await.unwrap();
    assert!(!docker.inspect_running(&id).await.unwrap());

    // Hard idle: remove
    docker.remove(&id, true).await.unwrap();

    let started = docker.started.lock().await.clone();
    let stopped = docker.stopped.lock().await.clone();
    let removed = docker.removed.lock().await.clone();

    assert_eq!(started, vec![id.clone()]);
    assert_eq!(stopped, vec![(id.clone(), 2)]);
    assert_eq!(removed, vec![(id.clone(), true)]);
}
