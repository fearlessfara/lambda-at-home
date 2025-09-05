use lambda_control::registry::ControlPlane;
use lambda_models::{Config, Function};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::collections::HashMap;

#[tokio::test]
async fn resolve_env_vars_resolves_secrets() {
    let config = Config::default();
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let invoker = Arc::new(lambda_invoker::Invoker::new(config.clone()).await.unwrap());
    let cp = ControlPlane::new(pool, invoker, config.clone()).await.unwrap();

    // Create a secret
    cp.create_secret("DB_PASS", "s3cr3t").await.unwrap();

    // Build a fake function referencing the secret
    let mut env: HashMap<String,String> = HashMap::new();
    env.insert("DATABASE_PASSWORD".into(), "SECRET_REF:DB_PASS".into());
    let f = Function {
        function_id: uuid::Uuid::new_v4(),
        function_name: "test".into(),
        runtime: "nodejs18.x".into(),
        role: None,
        handler: "index.handler".into(),
        code_sha256: "sha".into(),
        description: None,
        timeout: 3,
        memory_size: 128,
        environment: env,
        last_modified: chrono::Utc::now(),
        code_size: 0,
        version: "1".into(),
        state: lambda_models::FunctionState::Active,
        state_reason: None,
        state_reason_code: None,
    };

    let resolved = cp.resolve_env_vars(&f).await.unwrap();
    assert_eq!(resolved.get("DATABASE_PASSWORD").unwrap(), "s3cr3t");
}

