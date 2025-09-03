pub mod registry;
pub mod scheduler;
pub mod warm_pool;
pub mod concurrency;
pub mod idle_watchdog;
pub mod pending;
pub mod work_item;
pub mod queues;

pub use registry::*;
pub use scheduler::*;
pub use warm_pool::*;
pub use concurrency::*;
pub use idle_watchdog::*;
pub use pending::*;
pub use work_item::*;
pub use queues::*;

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_models::{Function, FunctionState, InvokeRequest, InvokeResponse};
    use std::time::{Duration, Instant};
    use uuid::Uuid;

    fn create_test_function() -> Function {
        Function {
            function_id: Uuid::new_v4(),
            function_name: "test-function".to_string(),
            runtime: "nodejs18.x".to_string(),
            role: None,
            handler: "index.handler".to_string(),
            code_sha256: "abcd1234".to_string(),
            description: None,
            timeout: 30,
            memory_size: 512,
            environment: std::collections::HashMap::new(),
            last_modified: chrono::Utc::now(),
            code_size: 1024,
            version: "1".to_string(),
            state: FunctionState::Active,
            state_reason: None,
            state_reason_code: None,
        }
    }

    #[tokio::test]
    async fn test_concurrency_token_bucket() {
        let manager = ConcurrencyManager::with_max_concurrency(2);
        let function = create_test_function();
        
        // Should be able to acquire 2 tokens
        let token1 = manager.acquire_token(&function).await.unwrap();
        let token2 = manager.acquire_token(&function).await.unwrap();
        
        // Third should fail (use non-blocking version for test)
        let result = manager.try_acquire_token(&function);
        assert!(result.is_err());
        
        // Release tokens
        drop(token1);
        drop(token2);
        
        // Should be able to acquire again
        let token3 = manager.acquire_token(&function).await.unwrap();
        drop(token3);
    }

    #[tokio::test]
    async fn test_warm_pool_keying() {
        let pool = WarmPool::new();
        let function = create_test_function();
        let env_hash = "test-env-hash";
        
        // Create FnKey for the test
        let key = crate::queues::FnKey {
            function_name: function.function_name.clone(),
            runtime: function.runtime.clone(),
            version: function.version.clone(),
            env_hash: env_hash.to_string(),
        };
        
        let container = WarmContainer {
            container_id: "test-container".to_string(),
            function_id: function.function_id,
            image_ref: "test-image".to_string(),
            env_hash: env_hash.to_string(),
            created_at: Instant::now(),
            last_used: Instant::now(),
            is_available: true,
        };
        
        pool.add_warm_container(key.clone(), container).await;
        
        // Should be able to get the container
        let retrieved = pool.get_warm_container(&key).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().container_id, "test-container");
        
        // Should not be available after retrieval
        let retrieved2 = pool.get_warm_container(&key).await;
        assert!(retrieved2.is_none());
    }

    #[tokio::test]
    async fn test_scheduler_invocation_lifecycle() {
        let (scheduler, _rx) = Scheduler::new();
        let function = create_test_function();
        
        let request = InvokeRequest {
            function_name: "test-function".to_string(),
            invocation_type: lambda_models::InvocationType::RequestResponse,
            log_type: Some(lambda_models::LogType::Tail),
            client_context: None,
            payload: Some(serde_json::json!({"test": "data"})),
            qualifier: None,
        };
        
        // Create a work item
        let req_id = uuid::Uuid::new_v4().to_string();
        let work_item = crate::work_item::WorkItem::from_invoke_request(req_id.clone(), function, request);
        
        // Register pending waiter
        let rx = scheduler.pending().register(req_id.clone());
        
        // Enqueue work item
        scheduler.enqueue(work_item).await.unwrap();
        
        // Complete invocation
        let response = InvokeResponse {
            status_code: 200,
            payload: Some(serde_json::json!({"result": "success"})),
            executed_version: Some("1".to_string()),
            function_error: None,
            log_result: None,
            headers: std::collections::HashMap::new(),
        };
        
        let payload = serde_json::to_vec(&response.payload.unwrap()).unwrap();
        let result = crate::pending::InvocationResult::ok(payload);
        let success = scheduler.pending().complete(&req_id, result);
        assert!(success);
        
        // Should receive the result
        let received = rx.await.unwrap();
        assert!(received.ok);
    }

    #[tokio::test]
    async fn test_idle_timers() {
        let pool = WarmPool::new();
        let function = create_test_function();
        
        // Create FnKey for the test
        let key = crate::queues::FnKey {
            function_name: function.function_name.clone(),
            runtime: function.runtime.clone(),
            version: function.version.clone(),
            env_hash: "test-env".to_string(),
        };
        
        let container = WarmContainer {
            container_id: "test-container".to_string(),
            function_id: function.function_id,
            image_ref: "test-image".to_string(),
            env_hash: "test-env".to_string(),
            created_at: Instant::now(),
            last_used: Instant::now() - Duration::from_secs(5), // 5 seconds ago
            is_available: true,
        };
        
        pool.add_warm_container(key.clone(), container).await;
        
        // Test cleanup with soft idle = 2s, hard idle = 10s
        let soft_idle = Duration::from_secs(2);
        let hard_idle = Duration::from_secs(10);
        
        let removed = pool.cleanup_idle_containers(soft_idle, hard_idle).await;
        assert_eq!(removed.len(), 0); // Should not be removed yet (only 5s old)
        
        // Test with very old container
        let old_container = WarmContainer {
            container_id: "old-container".to_string(),
            function_id: function.function_id,
            image_ref: "test-image".to_string(),
            env_hash: "test-env".to_string(),
            created_at: Instant::now(),
            last_used: Instant::now() - Duration::from_secs(15), // 15 seconds ago
            is_available: true,
        };
        
        pool.add_warm_container(key.clone(), old_container).await;
        
        let removed = pool.cleanup_idle_containers(soft_idle, hard_idle).await;
        assert_eq!(removed.len(), 1); // Should remove the old container
        assert_eq!(removed[0], "old-container");
    }
}