use lambda_models::Function;
use sqlx::SqlitePool;
use std::sync::Arc;

/// Handles execution tracking and database operations for Lambda invocations
#[derive(Clone)]
pub struct ExecutionTracker {
    pool: Arc<SqlitePool>,
}

impl ExecutionTracker {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }

    /// Record the start of an execution (deferred async)
    pub fn record_execution_start(
        &self,
        execution_id: String,
        function: &Function,
        aws_request_id: String,
        start_time: chrono::DateTime<chrono::Utc>,
    ) {
        let pool = self.pool.clone();
        let function_id = function.function_id;
        let function_version = function.version.clone();
        let execution_id_clone = execution_id.clone();

        tokio::spawn(async move {
            let _ = sqlx::query(
                "INSERT INTO executions (execution_id, function_id, function_version, aws_request_id, start_time, status)
                 VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&execution_id_clone)
            .bind(function_id)
            .bind(&function_version)
            .bind(&aws_request_id)
            .bind(start_time)
            .bind("Running")
            .execute(&*pool)
            .await;
        });
    }

    /// Record successful execution completion (deferred async)
    pub fn record_execution_success(
        &self,
        execution_id: String,
        end_time: chrono::DateTime<chrono::Utc>,
    ) {
        let pool = self.pool.clone();
        let execution_id_clone = execution_id.clone();

        tokio::spawn(async move {
            // Get the start time to calculate duration
            let start_time: Option<chrono::DateTime<chrono::Utc>> =
                sqlx::query_scalar("SELECT start_time FROM executions WHERE execution_id = ?")
                    .bind(&execution_id_clone)
                    .fetch_optional(&*pool)
                    .await
                    .unwrap_or(None);

            let duration_ms = if let Some(start) = start_time {
                (end_time - start).num_milliseconds()
            } else {
                0
            };

            let _ = sqlx::query(
                "UPDATE executions SET end_time = ?, status = 'Success', duration_ms = ? WHERE execution_id = ?"
            )
            .bind(end_time)
            .bind(duration_ms)
            .bind(&execution_id_clone)
            .execute(&*pool)
            .await;
        });
    }

    /// Record failed execution completion (deferred async)
    pub fn record_execution_failure(
        &self,
        execution_id: String,
        error_type: String,
        end_time: chrono::DateTime<chrono::Utc>,
    ) {
        let pool = self.pool.clone();
        let execution_id_clone = execution_id.clone();

        tokio::spawn(async move {
            // Get the start time to calculate duration
            let start_time: Option<chrono::DateTime<chrono::Utc>> =
                sqlx::query_scalar("SELECT start_time FROM executions WHERE execution_id = ?")
                    .bind(&execution_id_clone)
                    .fetch_optional(&*pool)
                    .await
                    .unwrap_or(None);

            let duration_ms = if let Some(start) = start_time {
                (end_time - start).num_milliseconds()
            } else {
                0
            };

            let _ = sqlx::query(
                "UPDATE executions SET end_time = ?, status = 'Failed', error_type = ?, duration_ms = ? WHERE execution_id = ?"
            )
            .bind(end_time)
            .bind(&error_type)
            .bind(duration_ms)
            .bind(&execution_id_clone)
            .execute(&*pool)
            .await;
        });
    }

    /// Record timeout execution (deferred async)
    pub fn record_execution_timeout(
        &self,
        execution_id: String,
        end_time: chrono::DateTime<chrono::Utc>,
    ) {
        self.record_execution_failure(execution_id, "TaskTimedOut".to_string(), end_time);
    }

    /// Record init error execution (deferred async)
    pub fn record_execution_init_error(
        &self,
        execution_id: String,
        end_time: chrono::DateTime<chrono::Utc>,
    ) {
        self.record_execution_failure(execution_id, "InitError".to_string(), end_time);
    }
}
