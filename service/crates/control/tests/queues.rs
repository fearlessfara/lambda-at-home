use lambda_control::queues::{FnKey, Queues};
use lambda_control::work_item::{FunctionMeta, WorkItem};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout, Duration};

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

fn sample_meta() -> FunctionMeta {
    FunctionMeta {
        function_name: "hello".into(),
        runtime: "nodejs18.x".into(),
        version: None,
        environment: Some(
            [
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "2".to_string()),
            ]
            .into_iter()
            .collect(),
        ),
        timeout_ms: 1500,
    }
}

fn wi(id: &str) -> WorkItem {
    WorkItem {
        request_id: id.to_string(),
        function: sample_meta(),
        payload: br#"{"ping":"pong"}"#.to_vec(),
        deadline_ms: now_ms() + 1500,
        log_type: None,
        client_context: None,
        cognito_identity: None,
    }
}

fn fn_key_from_meta() -> FnKey {
    let w = wi("test-key");
    FnKey::from_work_item(&w)
}

#[tokio::test]
async fn pop_blocks_then_returns_when_pushed() {
    let qs = Queues::new();
    let key = fn_key_from_meta();

    // Start waiter
    let qs_c = qs.clone();
    let key_c = key.clone();
    let waiter =
        tokio::spawn(
            async move { timeout(Duration::from_secs(2), qs_c.pop_or_wait(&key_c)).await },
        );

    // Give it time to park
    sleep(Duration::from_millis(50)).await;

    // Push work
    qs.push(wi("req-1")).unwrap();

    let got = waiter.await.unwrap().unwrap().unwrap();
    assert_eq!(got.request_id, "req-1");
}

#[tokio::test]
async fn fifo_order_is_preserved() {
    let qs = Queues::new();
    let key = fn_key_from_meta();

    qs.push(wi("r1")).unwrap();
    qs.push(wi("r2")).unwrap();
    qs.push(wi("r3")).unwrap();

    let a = timeout(Duration::from_millis(200), qs.pop_or_wait(&key))
        .await
        .unwrap()
        .unwrap();
    let b = timeout(Duration::from_millis(200), qs.pop_or_wait(&key))
        .await
        .unwrap()
        .unwrap();
    let c = timeout(Duration::from_millis(200), qs.pop_or_wait(&key))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(a.request_id, "r1");
    assert_eq!(b.request_id, "r2");
    assert_eq!(c.request_id, "r3");
}

#[tokio::test]
async fn lost_wakeup_is_avoided() {
    let qs = Queues::new();
    let key = fn_key_from_meta();

    // Spawn several waiters before any push
    let mut tasks = Vec::new();
    for _ in 0..3 {
        let qs_c = qs.clone();
        let k = key.clone();
        tasks.push(tokio::spawn(async move {
            qs_c.pop_or_wait(&k).await.unwrap().request_id
        }));
    }

    sleep(Duration::from_millis(50)).await;

    // Push 3 items; all waiters must resolve
    qs.push(wi("a")).unwrap();
    qs.push(wi("b")).unwrap();
    qs.push(wi("c")).unwrap();

    let mut ids: Vec<String> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["a", "b", "c"]);
}
