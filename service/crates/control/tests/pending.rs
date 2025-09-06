use lambda_control::pending::{InvocationResult, Pending};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn pending_delivers_once() {
    let p = Pending::new();
    let id = "req-42".to_string();
    let rx = p.register(id.clone());

    assert!(p.complete(&id, InvocationResult::ok(b"ok".to_vec())));

    let res = timeout(Duration::from_millis(200), rx)
        .await
        .unwrap()
        .unwrap();
    assert!(res.ok);
    assert_eq!(res.payload, b"ok".to_vec());

    // Late/duplicate completes should return false (no waiter)
    assert!(!p.complete(&id, InvocationResult::ok(b"late".to_vec())));
}
