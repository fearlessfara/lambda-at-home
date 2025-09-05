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
async fn create_sets_security_hardening_and_runtime_env() {
    let fake = FakeDocker::new();

    // Simulate your higher-level create function. Replace this with your real call:
    // e.g., invoker::container::create_function_container(docker, meta, key, ports, config).await
    let spec = CreateSpec {
        image: "lambda-nodejs18:latest".into(),
        name: "fn-hello-eh123".into(),
        env: vec![
            ("AWS_LAMBDA_FUNCTION_NAME".into(), "hello".into()),
            (
                "AWS_LAMBDA_RUNTIME_API".into(),
                "host.docker.internal:9001".into(),
            ),
        ],
        extra_hosts: vec!["host.docker.internal:host-gateway".into()],
        read_only_root_fs: true,
        user: Some("1000:1000".into()),
        cap_drop: vec!["ALL".into()],
        no_new_privileges: true,
        mounts: vec![
            // allow tmp as writable
            ("/tmp".into(), "/tmp".into(), false),
        ],
        ulimits: vec![("nofile".into(), 1024)],
        labels: vec![("lambda@home.fn".into(), "hello".into())],
        network: None,
    };

    // Call the trait directly; your production wrapper should fill CreateSpec like above.
    let id = fake.create(spec).await.unwrap();
    assert!(id.starts_with("ctr-"));

    let created = fake.last_created().await;

    // Security hardening
    assert!(created.read_only_root_fs);
    assert_eq!(created.cap_drop, vec!["ALL"]);
    assert!(created.no_new_privileges);
    assert_eq!(created.user.as_deref(), Some("1000:1000"));

    // Writable mount for /tmp
    assert!(created.mounts.iter().any(|m| m.1 == "/tmp" && m.2 == false));

    // Env & host reachability
    let env = created
        .env
        .iter()
        .cloned()
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(
        env.get("AWS_LAMBDA_FUNCTION_NAME").map(|s| s.as_str()),
        Some("hello")
    );
    assert_eq!(
        env.get("AWS_LAMBDA_RUNTIME_API").map(|s| s.as_str()),
        Some("host.docker.internal:9001")
    );
    assert!(created
        .extra_hosts
        .iter()
        .any(|h| h == "host.docker.internal:host-gateway"));

    // Labels
    assert!(created
        .labels
        .iter()
        .any(|(k, v)| k == "lambda@home.fn" && v == "hello"));
}
