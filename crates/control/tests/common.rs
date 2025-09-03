use lambda_control::work_item::{WorkItem, FunctionMeta};
use lambda_control::queues::FnKey;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

pub fn sample_meta() -> FunctionMeta {
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

pub fn wi(id: &str) -> WorkItem {
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

pub fn fn_key_from_meta() -> FnKey {
    // Hash must match FnKey::from_work_item(sample_meta()).
    // We can derive from a WorkItem to ensure consistency.
    let w = wi("test-key");
    FnKey::from_work_item(&w)
}
