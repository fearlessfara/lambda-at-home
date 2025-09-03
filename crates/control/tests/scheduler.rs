use lambda_control::queues::{Queues, FnKey};
use lambda_control::scheduler::{Scheduler, run_dispatcher};
use lambda_control::work_item::{WorkItem, FunctionMeta};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{timeout, Duration};

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

fn sample_meta() -> FunctionMeta {
    FunctionMeta {
        function_name: "hello".into(),
        runtime: "nodejs18.x".into(),
        version: None,
        environment: Some([
            ("A".to_string(), "1".to_string()),
            ("B".to_string(), "2".to_string()),
        ].into_iter().collect()),
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
async fn dispatcher_fans_out_to_queues() {
    let qs = Queues::new();
    let (sched, rx) = Scheduler::new();

    // Spawn dispatcher
    let d = tokio::spawn(run_dispatcher(rx, qs.clone()));

    // Enqueue work
    sched.enqueue(wi("S1")).await.unwrap();
    sched.enqueue(wi("S2")).await.unwrap();

    // Pop from per-fn queue
    let key = fn_key_from_meta();
    let a = timeout(Duration::from_millis(200), qs.pop_or_wait(&key)).await.unwrap().unwrap();
    let b = timeout(Duration::from_millis(200), qs.pop_or_wait(&key)).await.unwrap().unwrap();

    assert_eq!(a.request_id, "S1");
    assert_eq!(b.request_id, "S2");

    drop(d); // end dispatcher
}
