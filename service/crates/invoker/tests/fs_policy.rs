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
    async fn get_docker_stats(&self) -> anyhow::Result<lambda_models::DockerStats> {
        // Return a mock DockerStats for testing
        Ok(lambda_models::DockerStats {
            system_info: lambda_models::DockerSystemInfo {
                containers: 0,
                containers_running: 0,
                containers_paused: 0,
                containers_stopped: 0,
                images: 0,
                driver: "test".to_string(),
                memory_total: 0,
                memory_available: 0,
                cpu_count: 0,
                kernel_version: "test".to_string(),
                operating_system: "test".to_string(),
                architecture: "test".to_string(),
                docker_root_dir: "/test".to_string(),
                storage_driver: "test".to_string(),
                logging_driver: "test".to_string(),
                cgroup_driver: "test".to_string(),
                cgroup_version: "test".to_string(),
                n_events_listener: 0,
                n_goroutines: 0,
                system_time: "test".to_string(),
                server_version: "test".to_string(),
            },
            disk_usage: lambda_models::DockerDiskUsage {
                layers_size: 0,
                images: vec![],
                containers: vec![],
                volumes: vec![],
                build_cache: vec![],
            },
            version: lambda_models::DockerVersion {
                version: "test".to_string(),
                api_version: "test".to_string(),
                min_api_version: "test".to_string(),
                git_commit: "test".to_string(),
                go_version: "test".to_string(),
                os: "test".to_string(),
                arch: "test".to_string(),
                kernel_version: "test".to_string(),
                experimental: false,
                build_time: "test".to_string(),
            },
            cache_stats: None,
        })
    }
}

#[tokio::test]
async fn readonly_fs_with_writable_tmp_and_layers() {
    let docker = FakeDocker::new();

    let spec = CreateSpec {
        image: "lambda-python:latest".into(),
        name: "fn-py".into(),
        env: vec![],
        extra_hosts: vec![],
        read_only_root_fs: true,
        user: Some("1000:1000".into()),
        cap_drop: vec!["ALL".into()],
        no_new_privileges: true,
        mounts: vec![
            ("/tmp".into(), "/tmp".into(), false),
            ("/opt".into(), "/opt".into(), true), // layers read-only
        ],
        ulimits: vec![],
        labels: vec![],
        network: None,
    };

    docker.create(spec).await.unwrap();
    let created = docker.last_created().await;

    assert!(created.read_only_root_fs, "root fs must be read-only");
    assert!(
        created.mounts.iter().any(|m| m.1 == "/tmp" && !m.2),
        "/tmp must be writable"
    );
    assert!(
        created.mounts.iter().any(|m| m.1 == "/opt" && m.2),
        "/opt (layers) should be read-only"
    );
}
