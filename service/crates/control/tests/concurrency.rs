use lambda_control::concurrency::Concurrency;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn raii_permit_released_on_drop() {
    let c = Concurrency::new(1);

    // Acquire once
    let _g1 = c.acquire().await.unwrap();

    // Second acquire should block until first drops
    let c2 = c.clone();
    let mut waiter = tokio::spawn(async move { c2.acquire().await });

    // Still blocked after 50ms
    tokio::select! {
        _ = &mut waiter => panic!("Should not complete yet"),
        _ = tokio::time::sleep(Duration::from_millis(50)) => {}
    }

    drop(_g1);
    // Now it should complete
    let guard = timeout(Duration::from_secs(1), waiter)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    drop(guard);
}
