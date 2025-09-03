use lambda_control::queues::FnKey;
use lambda_control::work_item::{WorkItem, FunctionMeta};
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn env_hash_is_stable_regardless_of_map_order() {
    // Build two metas with same pairs but different insertion order
    use std::collections::HashMap;
    let mut m1 = HashMap::new(); 
    m1.insert("A".to_string(),"1".to_string()); 
    m1.insert("B".to_string(),"2".to_string());
    
    let mut m2 = HashMap::new(); 
    m2.insert("B".to_string(),"2".to_string()); 
    m2.insert("A".to_string(),"1".to_string());

    let mut meta1 = sample_meta(); 
    meta1.environment = Some(m1);
    let mut meta2 = sample_meta(); 
    meta2.environment = Some(m2);

    let w1 = lambda_control::work_item::WorkItem { 
        request_id: "x".into(), 
        function: meta1, 
        payload: vec![], 
        deadline_ms: 0, 
        log_type: None,
        client_context: None,
        cognito_identity: None,
    };
    let w2 = lambda_control::work_item::WorkItem { 
        request_id: "y".into(), 
        function: meta2, 
        payload: vec![], 
        deadline_ms: 0, 
        log_type: None,
        client_context: None,
        cognito_identity: None,
    };

    let k1 = FnKey::from_work_item(&w1);
    let k2 = FnKey::from_work_item(&w2);

    assert_eq!(k1.env_hash, k2.env_hash);
}

#[test]
fn version_defaults_to_latest() {
    let w = wi("z");
    let k = FnKey::from_work_item(&w);
    assert_eq!(k.version, "LATEST");
}
