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
