use sqlx::{Pool, Sqlite};
use tracing::info;

/// Embedded migration scripts
const MIGRATION_001_INITIAL: &str = include_str!("../migrations/001_initial.sql");
const MIGRATION_002_API_ROUTES: &str = include_str!("../migrations/002_api_routes.sql");
const MIGRATION_003_SECRETS: &str = include_str!("../migrations/003_secrets.sql");
const MIGRATION_004_FUNCTION_CONCURRENCY: &str = include_str!("../migrations/004_function_concurrency.sql");

/// Run all embedded migrations
pub async fn run_migrations(pool: &Pool<Sqlite>) -> Result<(), sqlx::Error> {
    info!("Running database migrations...");
    
    // Migration 001: Initial schema
    info!("Running migration 001: Initial schema");
    sqlx::query(MIGRATION_001_INITIAL)
        .execute(pool)
        .await?;
    
    // Migration 002: API Routes
    info!("Running migration 002: API Routes");
    sqlx::query(MIGRATION_002_API_ROUTES)
        .execute(pool)
        .await?;
    
    // Migration 003: Secrets
    info!("Running migration 003: Secrets");
    sqlx::query(MIGRATION_003_SECRETS)
        .execute(pool)
        .await?;
    
    // Migration 004: Function Concurrency
    info!("Running migration 004: Function Concurrency");
    sqlx::query(MIGRATION_004_FUNCTION_CONCURRENCY)
        .execute(pool)
        .await?;
    
    info!("All migrations completed successfully");
    Ok(())
}
