use lambda_control::warm_pool::{WarmPool, WarmContainer};
use lambda_control::queues::FnKey;
use std::time::Instant;
use uuid::Uuid;

#[tokio::test]
async fn warm_pool_basic_operations() {
    let pool = WarmPool::new();
    
    let key = FnKey {
        function_name: "test-fn".to_string(),
        runtime: "nodejs18.x".to_string(),
        version: "LATEST".to_string(),
        env_hash: "".to_string(),
    };
    
    let container = WarmContainer {
        container_id: "test-container".to_string(),
        instance_id: "inst-1".to_string(),
        function_id: Uuid::new_v4(),
        image_ref: "test-image".to_string(),
        created_at: Instant::now(),
        last_used: Instant::now(),
        state: lambda_control::warm_pool::InstanceState::WarmIdle,
    };
    
    // Add container to pool
    pool.add_warm_container(key.clone(), container.clone()).await;
    
    // Should be able to get it back
    let retrieved = pool.get_warm_container(&key).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().container_id, "test-container");
    
    // Should not be available anymore
    let retrieved2 = pool.get_warm_container(&key).await;
    assert!(retrieved2.is_none());
}

#[tokio::test]
async fn drain_by_function_id_removes_all() {
    let pool = WarmPool::new();

    let key_a = FnKey { function_name: "fn-a".into(), runtime: "nodejs18.x".into(), version: "LATEST".into(), env_hash: "h1".into() };
    let key_b = FnKey { function_name: "fn-a".into(), runtime: "nodejs18.x".into(), version: "LATEST".into(), env_hash: "h2".into() };
    let fid = Uuid::new_v4();

    for (i, key) in [key_a.clone(), key_b.clone()].into_iter().enumerate() {
        let c = WarmContainer {
            container_id: format!("c{}", i+1),
            instance_id: format!("inst-{}", i+1),
            function_id: fid,
            image_ref: "img".into(),
            created_at: Instant::now(),
            last_used: Instant::now(),
            state: lambda_control::warm_pool::InstanceState::WarmIdle,
        };
        pool.add_warm_container(key, c).await;
    }

    let removed = pool.drain_by_function_id(fid).await;
    assert_eq!(removed.len(), 2);
    assert_eq!(pool.container_count(&key_a).await, 0);
    assert_eq!(pool.container_count(&key_b).await, 0);
}
