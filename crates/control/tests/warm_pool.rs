use lambda_control::warm_pool::{WarmPool, WarmContainer};
use lambda_control::queues::FnKey;
use std::time::Instant;
use uuid::Uuid;

#[tokio::test]
async fn warm_pool_basic_operations() {
    let pool = WarmPool::new();
    
    let key = FnKey {
        function_name: "test-fn".to_string(),
    };
    
    let container = WarmContainer {
        container_id: "test-container".to_string(),
        function_id: Uuid::new_v4(),
        image_ref: "test-image".to_string(),
        created_at: Instant::now(),
        last_used: Instant::now(),
        is_available: true,
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
