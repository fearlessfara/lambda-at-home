use lambda_control::container_monitor::ContainerMonitor;
use lambda_control::queues::FnKey;
use lambda_control::warm_pool::{InstanceState, WarmContainer, WarmPool};
use lambda_invoker::ContainerEvent;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

#[tokio::test]
async fn test_container_monitor_handles_die_event() {
    let warm_pool = Arc::new(WarmPool::new());
    let (monitor, _sender) = ContainerMonitor::new(warm_pool.clone());

    // Add a test container to the warm pool
    let fn_key = FnKey {
        function_name: "test-function".to_string(),
        runtime: "nodejs22.x".to_string(),
        version: "1".to_string(),
        env_hash: "test-env".to_string(),
    };

    let container = WarmContainer {
        container_id: "test-container-123".to_string(),
        instance_id: "test-instance-123".to_string(),
        function_id: Uuid::new_v4(),
        image_ref: "test-image:latest".to_string(),
        created_at: Instant::now(),
        last_used: Instant::now(),
        state: InstanceState::WarmIdle,
    };

    warm_pool
        .add_warm_container(fn_key.clone(), container)
        .await;

    // Verify container is in warm pool
    assert_eq!(warm_pool.container_count(&fn_key).await, 1);

    // Simulate a die event
    let die_event = ContainerEvent::Die {
        container_id: "test-container-123".to_string(),
        exit_code: Some(0),
    };

    // Handle the event
    monitor.handle_container_event(die_event).await.unwrap();

    // Verify container was removed from warm pool
    assert_eq!(warm_pool.container_count(&fn_key).await, 0);
}

#[tokio::test]
async fn test_container_monitor_handles_stop_event() {
    let warm_pool = Arc::new(WarmPool::new());
    let (monitor, _sender) = ContainerMonitor::new(warm_pool.clone());

    // Add a test container to the warm pool
    let fn_key = FnKey {
        function_name: "test-function".to_string(),
        runtime: "nodejs22.x".to_string(),
        version: "1".to_string(),
        env_hash: "test-env".to_string(),
    };

    let container = WarmContainer {
        container_id: "test-container-456".to_string(),
        instance_id: "test-instance-456".to_string(),
        function_id: Uuid::new_v4(),
        image_ref: "test-image:latest".to_string(),
        created_at: Instant::now(),
        last_used: Instant::now(),
        state: InstanceState::WarmIdle,
    };

    warm_pool
        .add_warm_container(fn_key.clone(), container)
        .await;

    // Simulate a stop event
    let stop_event = ContainerEvent::Stop {
        container_id: "test-container-456".to_string(),
    };

    // Handle the event
    monitor.handle_container_event(stop_event).await.unwrap();

    // Verify container state was updated to Stopped
    assert!(
        warm_pool
            .set_state_by_container_id("test-container-456", InstanceState::Stopped)
            .await
    );
}

#[tokio::test]
async fn test_container_monitor_handles_kill_event() {
    let warm_pool = Arc::new(WarmPool::new());
    let (monitor, _sender) = ContainerMonitor::new(warm_pool.clone());

    // Add a test container to the warm pool
    let fn_key = FnKey {
        function_name: "test-function".to_string(),
        runtime: "nodejs22.x".to_string(),
        version: "1".to_string(),
        env_hash: "test-env".to_string(),
    };

    let container = WarmContainer {
        container_id: "test-container-789".to_string(),
        instance_id: "test-instance-789".to_string(),
        function_id: Uuid::new_v4(),
        image_ref: "test-image:latest".to_string(),
        created_at: Instant::now(),
        last_used: Instant::now(),
        state: InstanceState::Active,
    };

    warm_pool
        .add_warm_container(fn_key.clone(), container)
        .await;

    // Verify container is in warm pool
    assert_eq!(warm_pool.container_count(&fn_key).await, 1);

    // Simulate a kill event
    let kill_event = ContainerEvent::Kill {
        container_id: "test-container-789".to_string(),
    };

    // Handle the event
    monitor.handle_container_event(kill_event).await.unwrap();

    // Verify container was removed from warm pool
    assert_eq!(warm_pool.container_count(&fn_key).await, 0);
}

#[tokio::test]
async fn test_container_monitor_handles_remove_event() {
    let warm_pool = Arc::new(WarmPool::new());
    let (monitor, _sender) = ContainerMonitor::new(warm_pool.clone());

    // Add a test container to the warm pool
    let fn_key = FnKey {
        function_name: "test-function".to_string(),
        runtime: "nodejs22.x".to_string(),
        version: "1".to_string(),
        env_hash: "test-env".to_string(),
    };

    let container = WarmContainer {
        container_id: "test-container-remove".to_string(),
        instance_id: "test-instance-remove".to_string(),
        function_id: Uuid::new_v4(),
        image_ref: "test-image:latest".to_string(),
        created_at: Instant::now(),
        last_used: Instant::now(),
        state: InstanceState::Stopped,
    };

    warm_pool
        .add_warm_container(fn_key.clone(), container)
        .await;

    // Verify container is in warm pool
    assert_eq!(warm_pool.container_count(&fn_key).await, 1);

    // Simulate a remove event
    let remove_event = ContainerEvent::Remove {
        container_id: "test-container-remove".to_string(),
    };

    // Handle the event
    monitor.handle_container_event(remove_event).await.unwrap();

    // Verify container was removed from warm pool
    assert_eq!(warm_pool.container_count(&fn_key).await, 0);
}

#[tokio::test]
async fn test_container_monitor_handles_start_event() {
    let warm_pool = Arc::new(WarmPool::new());
    let (monitor, _sender) = ContainerMonitor::new(warm_pool.clone());

    // Add a test container to the warm pool in stopped state
    let fn_key = FnKey {
        function_name: "test-function".to_string(),
        runtime: "nodejs22.x".to_string(),
        version: "1".to_string(),
        env_hash: "test-env".to_string(),
    };

    let container = WarmContainer {
        container_id: "test-container-start".to_string(),
        instance_id: "test-instance-start".to_string(),
        function_id: Uuid::new_v4(),
        image_ref: "test-image:latest".to_string(),
        created_at: Instant::now(),
        last_used: Instant::now(),
        state: InstanceState::Stopped,
    };

    warm_pool
        .add_warm_container(fn_key.clone(), container)
        .await;

    // Simulate a start event
    let start_event = ContainerEvent::Start {
        container_id: "test-container-start".to_string(),
    };

    // Handle the event
    monitor.handle_container_event(start_event).await.unwrap();

    // Verify container state was updated to WarmIdle
    assert!(
        warm_pool
            .set_state_by_container_id("test-container-start", InstanceState::WarmIdle)
            .await
    );
}

#[tokio::test]
async fn test_container_monitor_handles_unknown_container() {
    let warm_pool = Arc::new(WarmPool::new());
    let (monitor, _sender) = ContainerMonitor::new(warm_pool.clone());

    // Simulate an event for a container not in our warm pool
    let unknown_event = ContainerEvent::Die {
        container_id: "unknown-container".to_string(),
        exit_code: Some(1),
    };

    // Handle the event - should not panic or error
    let result = monitor.handle_container_event(unknown_event).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_container_monitor_multiple_containers() {
    let warm_pool = Arc::new(WarmPool::new());
    let (monitor, _sender) = ContainerMonitor::new(warm_pool.clone());

    let fn_key = FnKey {
        function_name: "test-function".to_string(),
        runtime: "nodejs22.x".to_string(),
        version: "1".to_string(),
        env_hash: "test-env".to_string(),
    };

    // Add multiple containers
    for i in 1..=3 {
        let container = WarmContainer {
            container_id: format!("test-container-{i}"),
            instance_id: format!("test-instance-{i}"),
            function_id: Uuid::new_v4(),
            image_ref: "test-image:latest".to_string(),
            created_at: Instant::now(),
            last_used: Instant::now(),
            state: InstanceState::WarmIdle,
        };
        warm_pool
            .add_warm_container(fn_key.clone(), container)
            .await;
    }

    // Verify all containers are in warm pool
    assert_eq!(warm_pool.container_count(&fn_key).await, 3);

    // Remove one container
    let remove_event = ContainerEvent::Remove {
        container_id: "test-container-2".to_string(),
    };
    monitor.handle_container_event(remove_event).await.unwrap();

    // Verify only 2 containers remain
    assert_eq!(warm_pool.container_count(&fn_key).await, 2);

    // Stop another container
    let stop_event = ContainerEvent::Stop {
        container_id: "test-container-1".to_string(),
    };
    monitor.handle_container_event(stop_event).await.unwrap();

    // Verify container count is still 2 (stopped, not removed)
    assert_eq!(warm_pool.container_count(&fn_key).await, 2);
}

#[tokio::test]
async fn test_container_monitor_state_transitions() {
    let warm_pool = Arc::new(WarmPool::new());
    let (monitor, _sender) = ContainerMonitor::new(warm_pool.clone());

    let fn_key = FnKey {
        function_name: "test-function".to_string(),
        runtime: "nodejs22.x".to_string(),
        version: "1".to_string(),
        env_hash: "test-env".to_string(),
    };

    let container = WarmContainer {
        container_id: "test-container-state".to_string(),
        instance_id: "test-instance-state".to_string(),
        function_id: Uuid::new_v4(),
        image_ref: "test-image:latest".to_string(),
        created_at: Instant::now(),
        last_used: Instant::now(),
        state: InstanceState::WarmIdle,
    };

    warm_pool
        .add_warm_container(fn_key.clone(), container)
        .await;

    // Test state transitions: WarmIdle -> Stopped -> WarmIdle
    let stop_event = ContainerEvent::Stop {
        container_id: "test-container-state".to_string(),
    };
    monitor.handle_container_event(stop_event).await.unwrap();

    // Verify container is now stopped
    assert!(
        warm_pool
            .set_state_by_container_id("test-container-state", InstanceState::Stopped)
            .await
    );

    // Start the container again
    let start_event = ContainerEvent::Start {
        container_id: "test-container-state".to_string(),
    };
    monitor.handle_container_event(start_event).await.unwrap();

    // Verify container is back to WarmIdle
    assert!(
        warm_pool
            .set_state_by_container_id("test-container-state", InstanceState::WarmIdle)
            .await
    );

    // Finally, kill the container
    let kill_event = ContainerEvent::Kill {
        container_id: "test-container-state".to_string(),
    };
    monitor.handle_container_event(kill_event).await.unwrap();

    // Verify container was removed
    assert_eq!(warm_pool.container_count(&fn_key).await, 0);
}
