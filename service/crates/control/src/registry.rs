use crate::autoscaler::Autoscaler;
use crate::cache::FunctionCache;
use crate::concurrency::ConcurrencyManager;
use crate::execution_tracker::ExecutionTracker;
use crate::migrations;
use crate::pending::Pending;
use crate::queues::Queues;
use crate::scheduler::{run_dispatcher, Scheduler};
use crate::warm_pool::WarmPool;
use base64;
use chrono::Utc;
use lambda_models::{
    Alias, ApiRoute, CacheStats, CacheTypeStats, ConcurrencyConfig, CreateAliasRequest,
    CreateApiRouteRequest, CreateFunctionRequest, DockerStats, Function, FunctionError,
    FunctionState, InitError, InvokeRequest, InvokeResponse, LambdaError, ListAliasesResponse,
    ListApiRoutesResponse, ListFunctionsResponse, ListVersionsResponse, PublishVersionRequest,
    RoutingConfig, RuntimeError, RuntimeInvocation, RuntimeResponse, UpdateAliasRequest,
    UpdateFunctionCodeRequest, UpdateFunctionConfigurationRequest, Version,
};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use uuid::Uuid;

use crate::work_item::WorkItem;
// No need for FnKey import - using function names directly
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

pub struct ControlPlane {
    pool: SqlitePool,
    scheduler: Arc<Scheduler>,
    warm_pool: Arc<WarmPool>,
    concurrency_manager: Arc<ConcurrencyManager>,
    invoker: Arc<lambda_invoker::Invoker>,
    config: lambda_models::Config,
    cache: Arc<FunctionCache>,
    execution_tracker: ExecutionTracker,
}

impl ControlPlane {
    pub async fn new(
        pool: SqlitePool,
        invoker: Arc<lambda_invoker::Invoker>,
        config: lambda_models::Config,
    ) -> Result<Self, LambdaError> {
        // Run embedded migrations
        migrations::run_migrations(&pool)
            .await
            .map_err(|e| LambdaError::DatabaseError {
                reason: e.to_string(),
            })?;

        let (scheduler, rx) = Scheduler::new();
        let warm_pool = Arc::new(WarmPool::new());
        let concurrency_manager = Arc::new(ConcurrencyManager::new());

        // Initialize cache with configurable TTL (default 5 minutes)
        let cache_ttl = std::time::Duration::from_secs(300);
        let cache = Arc::new(FunctionCache::new(cache_ttl, 1000));

        // Spawn the dispatcher task
        let queues = scheduler.queues();
        tokio::spawn(async move {
            run_dispatcher(rx, queues).await;
        });

        // Spawn autoscaler loop
        let execution_tracker = ExecutionTracker::new(Arc::new(pool.clone()));
        let control_ref = Arc::new(Self {
            pool: pool.clone(),
            scheduler: Arc::new(scheduler.clone()),
            warm_pool: warm_pool.clone(),
            concurrency_manager: concurrency_manager.clone(),
            invoker: invoker.clone(),
            config: config.clone(),
            cache: cache.clone(),
            execution_tracker: execution_tracker.clone(),
        });
        let autoscaler = Autoscaler::new(control_ref.clone());
        tokio::spawn(async move {
            autoscaler.start().await;
        });

        // Start cache cleanup task
        let cache_cleanup = cache.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let cleaned = cache_cleanup.cleanup_expired();
                if cleaned > 0 {
                    debug!("Cache cleanup: removed {} expired entries", cleaned);
                }
            }
        });

        Ok(Self {
            pool,
            scheduler: Arc::new(scheduler),
            warm_pool,
            concurrency_manager,
            invoker,
            config,
            cache,
            execution_tracker,
        })
    }

    // Accessors for subsystems
    pub fn warm_pool(&self) -> Arc<WarmPool> {
        self.warm_pool.clone()
    }
    pub fn pending(&self) -> Pending {
        self.scheduler.pending()
    }
    pub fn invoker(&self) -> Arc<lambda_invoker::Invoker> {
        self.invoker.clone()
    }
    pub fn config(&self) -> lambda_models::Config {
        self.config.clone()
    }
    pub fn queues(&self) -> Queues {
        self.scheduler.queues()
    }

    // ---------------- API Gateway Route Management ----------------
    pub async fn create_api_route(
        &self,
        req: CreateApiRouteRequest,
    ) -> Result<ApiRoute, LambdaError> {
        if !self.function_exists(&req.function_name).await? {
            return Err(LambdaError::FunctionNotFound {
                function_name: req.function_name,
            });
        }
        let route_id = Uuid::new_v4();
        let created_at = chrono::Utc::now();
        let path = normalize_path(&req.path);
        let method = req.method.as_ref().map(|m| m.to_uppercase());

        sqlx::query(
            "INSERT INTO api_routes (route_id, path, method, function_name, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(route_id)
        .bind(&path)
        .bind(&method)
        .bind(&req.function_name)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;

        Ok(ApiRoute {
            route_id,
            path,
            method,
            function_name: req.function_name,
            created_at,
        })
    }

    pub async fn list_api_routes(&self) -> Result<ListApiRoutesResponse, LambdaError> {
        let rows = sqlx::query("SELECT * FROM api_routes ORDER BY path")
            .fetch_all(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;
        let mut routes = Vec::with_capacity(rows.len());
        for row in rows.iter() {
            routes.push(ApiRoute {
                route_id: row.try_get("route_id").map_err(LambdaError::SqlxError)?,
                path: row.try_get("path").map_err(LambdaError::SqlxError)?,
                method: row.try_get("method").map_err(LambdaError::SqlxError)?,
                function_name: row
                    .try_get("function_name")
                    .map_err(LambdaError::SqlxError)?,
                created_at: row.try_get("created_at").map_err(LambdaError::SqlxError)?,
            });
        }
        Ok(ListApiRoutesResponse { routes })
    }

    pub async fn delete_api_route(&self, route_id: Uuid) -> Result<(), LambdaError> {
        let result = sqlx::query("DELETE FROM api_routes WHERE route_id = ?")
            .bind(route_id)
            .execute(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;
        if result.rows_affected() == 0 {
            return Err(LambdaError::InvalidRequest {
                reason: "Route not found".to_string(),
            });
        }
        Ok(())
    }

    pub async fn resolve_api_route(
        &self,
        method: &str,
        path: &str,
    ) -> Result<Option<String>, LambdaError> {
        let norm_path = normalize_path(path);
        let upper_m = method.to_uppercase();
        let rows = sqlx::query("SELECT path, method, function_name FROM api_routes")
            .fetch_all(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;
        let mut best: Option<(usize, String)> = None;
        for row in rows.iter() {
            let r_path: String = row.try_get("path").map_err(LambdaError::SqlxError)?;
            let r_method: Option<String> = row.try_get("method").map_err(LambdaError::SqlxError)?;
            let fname: String = row
                .try_get("function_name")
                .map_err(LambdaError::SqlxError)?;
            if !norm_path.starts_with(&r_path) {
                continue;
            }
            if let Some(m) = r_method.as_ref() {
                if m.to_uppercase() != upper_m {
                    continue;
                }
            }
            let plen = r_path.len();
            if best.as_ref().map(|(l, _)| *l).unwrap_or(0) < plen {
                best = Some((plen, fname));
            }
        }
        Ok(best.map(|(_, f)| f))
    }

    #[instrument(skip(self, request))]
    pub async fn create_function(
        &self,
        request: CreateFunctionRequest,
    ) -> Result<Function, LambdaError> {
        let function_id = Uuid::new_v4();
        let now = Utc::now();

        // Validate function name
        if !self.is_valid_function_name(&request.function_name) {
            return Err(LambdaError::InvalidFunctionName {
                function_name: request.function_name,
            });
        }

        // Check if function already exists
        if self.function_exists(&request.function_name).await? {
            return Err(LambdaError::FunctionAlreadyExists {
                function_name: request.function_name,
            });
        }

        // Validate runtime
        if !self.is_valid_runtime(&request.runtime) {
            return Err(LambdaError::InvalidRuntime {
                runtime: request.runtime,
            });
        }

        // Process ZIP file if provided
        let (code_sha256, code_size, state) = if let Some(zip_file_base64) = &request.code.zip_file
        {
            let zip_data =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, zip_file_base64)
                    .map_err(|e| LambdaError::InvalidRequest {
                        reason: format!("Invalid base64 ZIP data: {e}"),
                    })?;

            // Process the ZIP file
            let packaging_service = lambda_packaging::PackagingService::new(self.config.clone());
            let zip_info = packaging_service.process_zip(&zip_data).await?;

            // Store the ZIP file
            packaging_service.store_zip(&zip_info)?;

            (zip_info.sha256, zip_info.total_size, FunctionState::Active)
        } else {
            ("".to_string(), 0, FunctionState::Pending)
        };

        // Create function record
        let function = Function {
            function_id,
            function_name: request.function_name.clone(),
            runtime: request.runtime,
            role: request.role,
            handler: request.handler,
            code_sha256,
            description: request.description,
            timeout: request.timeout.unwrap_or(3), // seconds, not milliseconds
            memory_size: request.memory_size.unwrap_or(512),
            environment: request.environment.unwrap_or_default(),
            last_modified: now,
            code_size,
            version: "1".to_string(),
            state,
            state_reason: None,
            state_reason_code: None,
        };

        sqlx::query(
            r#"
            INSERT INTO functions (
                function_id, function_name, runtime, role, handler, code_sha256,
                description, timeout, memory_size, environment, last_modified,
                code_size, version, state, state_reason, state_reason_code
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(function.function_id)
        .bind(&function.function_name)
        .bind(&function.runtime)
        .bind(&function.role)
        .bind(&function.handler)
        .bind(&function.code_sha256)
        .bind(&function.description)
        .bind(function.timeout as i64)
        .bind(function.memory_size as i64)
        .bind(serde_json::to_string(&function.environment).unwrap_or_default())
        .bind(function.last_modified)
        .bind(function.code_size as i64)
        .bind(&function.version)
        .bind(serde_json::to_string(&function.state).unwrap_or_default())
        .bind(&function.state_reason)
        .bind(&function.state_reason_code)
        .execute(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;

        info!(
            "Created function: {} with code SHA256: {}",
            function.function_name, function.code_sha256
        );
        Ok(function)
    }

    #[instrument(skip(self))]
    pub async fn get_function(&self, name: &str) -> Result<Function, LambdaError> {
        // Try cache first
        if let Some(cached_function) = self.cache.get_function(name) {
            return Ok((*cached_function).clone());
        }

        // Cache miss - fetch from database
        let row = sqlx::query("SELECT * FROM functions WHERE function_name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?
            .ok_or_else(|| LambdaError::FunctionNotFound {
                function_name: name.to_string(),
            })?;

        let function = self.row_to_function(&row)?;

        // Cache the result
        self.cache.set_function(name.to_string(), function.clone());

        Ok(function)
    }

    #[instrument(skip(self))]
    pub async fn delete_function(&self, name: &str) -> Result<(), LambdaError> {
        // Get function first to get function_id for cache invalidation
        let function = self.get_function(name).await.ok();

        let result = sqlx::query("DELETE FROM functions WHERE function_name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;

        if result.rows_affected() == 0 {
            return Err(LambdaError::FunctionNotFound {
                function_name: name.to_string(),
            });
        }

        // Invalidate cache
        self.cache.invalidate_function(name);
        if let Some(func) = function {
            self.cache
                .invalidate_concurrency(&func.function_id.to_string());
            self.cache
                .invalidate_env_vars(&func.function_id.to_string());
        }

        info!("Deleted function: {}", name);
        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn list_functions(
        &self,
        marker: Option<&String>,
        max_items: Option<u32>,
    ) -> Result<ListFunctionsResponse, LambdaError> {
        let limit = max_items.unwrap_or(50).min(1000) as i64;
        let offset = marker.and_then(|m| m.parse::<i64>().ok()).unwrap_or(0);

        let rows = sqlx::query("SELECT * FROM functions ORDER BY function_name LIMIT ? OFFSET ?")
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;

        let functions: Result<Vec<Function>, LambdaError> =
            rows.iter().map(|row| self.row_to_function(row)).collect();

        let next_marker = if rows.len() as i64 == limit {
            Some((offset + limit).to_string())
        } else {
            None
        };

        Ok(ListFunctionsResponse {
            functions: functions?,
            next_marker,
        })
    }

    #[instrument(skip(self, _request))]
    pub async fn update_function_code(
        &self,
        name: &str,
        _request: UpdateFunctionCodeRequest,
    ) -> Result<Function, LambdaError> {
        // TODO: Implement code update logic
        let mut function = self.get_function(name).await?;
        function.last_modified = Utc::now();

        sqlx::query("UPDATE functions SET last_modified = ? WHERE function_name = ?")
            .bind(function.last_modified)
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;

        // Invalidate cache since function was updated
        self.cache.invalidate_function(name);
        self.cache
            .invalidate_env_vars(&function.function_id.to_string());

        Ok(function)
    }

    #[instrument(skip(self, request))]
    pub async fn update_function_configuration(
        &self,
        name: &str,
        request: UpdateFunctionConfigurationRequest,
    ) -> Result<Function, LambdaError> {
        let mut function = self.get_function(name).await?;
        let env_will_change = request.environment.is_some();

        if let Some(role) = request.role {
            function.role = Some(role);
        }
        if let Some(handler) = request.handler {
            function.handler = handler;
        }
        if let Some(description) = request.description {
            function.description = Some(description);
        }
        if let Some(timeout) = request.timeout {
            function.timeout = timeout;
        }
        if let Some(memory_size) = request.memory_size {
            function.memory_size = memory_size;
        }
        if let Some(environment) = request.environment {
            function.environment = environment;
        }

        function.last_modified = Utc::now();

        sqlx::query(
            r#"
            UPDATE functions SET 
                role = ?, handler = ?, description = ?, timeout = ?, 
                memory_size = ?, environment = ?, last_modified = ?
            WHERE function_name = ?
            "#,
        )
        .bind(&function.role)
        .bind(&function.handler)
        .bind(&function.description)
        .bind(function.timeout as i64)
        .bind(function.memory_size as i64)
        .bind(serde_json::to_string(&function.environment).unwrap_or_default())
        .bind(function.last_modified)
        .bind(name)
        .execute(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;

        // If env is updated, drain existing warm containers so new env applies immediately
        if env_will_change {
            let ids = self
                .warm_pool
                .drain_by_function_id(function.function_id)
                .await;
            for id in ids {
                let _ = self.invoker.remove_container(&id).await;
            }
        }

        // Invalidate cache since function configuration was updated
        self.cache.invalidate_function(name);
        self.cache
            .invalidate_env_vars(&function.function_id.to_string());

        Ok(function)
    }

    #[instrument(skip(self, request))]
    pub async fn publish_version(
        &self,
        name: &str,
        request: PublishVersionRequest,
    ) -> Result<Version, LambdaError> {
        let function = self.get_function(name).await?;
        let version_id = Uuid::new_v4();
        let now = Utc::now();

        let version = Version {
            version_id,
            function_id: function.function_id,
            version: "2".to_string(), // TODO: Implement proper versioning
            description: request.description,
            code_sha256: function.code_sha256,
            last_modified: now,
            code_size: function.code_size,
        };

        sqlx::query(
            r#"
            INSERT INTO versions (
                version_id, function_id, version, description, code_sha256,
                last_modified, code_size
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(version.version_id)
        .bind(version.function_id)
        .bind(&version.version)
        .bind(&version.description)
        .bind(&version.code_sha256)
        .bind(version.last_modified)
        .bind(version.code_size as i64)
        .execute(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;

        Ok(version)
    }

    #[instrument(skip(self))]
    pub async fn list_versions(
        &self,
        name: &str,
        marker: Option<&String>,
        max_items: Option<u32>,
    ) -> Result<ListVersionsResponse, LambdaError> {
        let function = self.get_function(name).await?;
        let limit = max_items.unwrap_or(50).min(1000) as i64;
        let offset = marker.and_then(|m| m.parse::<i64>().ok()).unwrap_or(0);

        let rows = sqlx::query(
            "SELECT * FROM versions WHERE function_id = ? ORDER BY version LIMIT ? OFFSET ?",
        )
        .bind(function.function_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;

        let versions: Result<Vec<Version>, LambdaError> =
            rows.iter().map(|row| self.row_to_version(row)).collect();

        let next_marker = if rows.len() as i64 == limit {
            Some((offset + limit).to_string())
        } else {
            None
        };

        Ok(ListVersionsResponse {
            versions: versions?,
            next_marker,
        })
    }

    #[instrument(skip(self, request))]
    pub async fn create_alias(
        &self,
        name: &str,
        request: CreateAliasRequest,
    ) -> Result<Alias, LambdaError> {
        let function = self.get_function(name).await?;
        let alias_id = Uuid::new_v4();
        let now = Utc::now();

        let alias = Alias {
            alias_id,
            function_id: function.function_id,
            name: request.name,
            function_version: request.function_version,
            description: request.description,
            routing_config: request.routing_config,
            revision_id: Uuid::new_v4().to_string(),
            last_modified: now,
        };

        sqlx::query(
            r#"
            INSERT INTO aliases (
                alias_id, function_id, name, function_version, description,
                routing_config, revision_id, last_modified
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(alias.alias_id)
        .bind(alias.function_id)
        .bind(&alias.name)
        .bind(&alias.function_version)
        .bind(&alias.description)
        .bind(serde_json::to_string(&alias.routing_config).unwrap_or_default())
        .bind(&alias.revision_id)
        .bind(alias.last_modified)
        .execute(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;

        Ok(alias)
    }

    #[instrument(skip(self))]
    pub async fn get_alias(&self, name: &str, alias: &str) -> Result<Alias, LambdaError> {
        let function = self.get_function(name).await?;

        let row = sqlx::query("SELECT * FROM aliases WHERE function_id = ? AND name = ?")
            .bind(function.function_id)
            .bind(alias)
            .fetch_optional(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?
            .ok_or_else(|| LambdaError::FunctionNotFound {
                function_name: format!("{name}/{alias}"),
            })?;

        self.row_to_alias(&row)
    }

    #[instrument(skip(self))]
    pub async fn update_alias(
        &self,
        name: &str,
        alias: &str,
        request: UpdateAliasRequest,
    ) -> Result<Alias, LambdaError> {
        let mut alias_obj = self.get_alias(name, alias).await?;

        if let Some(function_version) = request.function_version {
            alias_obj.function_version = function_version;
        }
        if let Some(description) = request.description {
            alias_obj.description = Some(description);
        }
        if let Some(routing_config) = request.routing_config {
            alias_obj.routing_config = Some(routing_config);
        }

        alias_obj.revision_id = Uuid::new_v4().to_string();
        alias_obj.last_modified = Utc::now();

        sqlx::query(
            r#"
            UPDATE aliases SET 
                function_version = ?, description = ?, routing_config = ?,
                revision_id = ?, last_modified = ?
            WHERE function_id = ? AND name = ?
            "#,
        )
        .bind(&alias_obj.function_version)
        .bind(&alias_obj.description)
        .bind(serde_json::to_string(&alias_obj.routing_config).unwrap_or_default())
        .bind(&alias_obj.revision_id)
        .bind(alias_obj.last_modified)
        .bind(alias_obj.function_id)
        .bind(alias)
        .execute(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;

        Ok(alias_obj)
    }

    #[instrument(skip(self))]
    pub async fn delete_alias(&self, name: &str, alias: &str) -> Result<(), LambdaError> {
        let function = self.get_function(name).await?;

        let result = sqlx::query("DELETE FROM aliases WHERE function_id = ? AND name = ?")
            .bind(function.function_id)
            .bind(alias)
            .execute(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;

        if result.rows_affected() == 0 {
            return Err(LambdaError::FunctionNotFound {
                function_name: format!("{name}/{alias}"),
            });
        }

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn list_aliases(
        &self,
        name: &str,
        marker: Option<&String>,
        max_items: Option<u32>,
    ) -> Result<ListAliasesResponse, LambdaError> {
        let function = self.get_function(name).await?;
        let limit = max_items.unwrap_or(50).min(1000) as i64;
        let offset = marker.and_then(|m| m.parse::<i64>().ok()).unwrap_or(0);

        let rows = sqlx::query(
            "SELECT * FROM aliases WHERE function_id = ? ORDER BY name LIMIT ? OFFSET ?",
        )
        .bind(function.function_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;

        let aliases: Result<Vec<Alias>, LambdaError> =
            rows.iter().map(|row| self.row_to_alias(row)).collect();

        let next_marker = if rows.len() as i64 == limit {
            Some((offset + limit).to_string())
        } else {
            None
        };

        Ok(ListAliasesResponse {
            aliases: aliases?,
            next_marker,
        })
    }

    #[instrument(skip(self))]
    pub async fn put_concurrency(
        &self,
        name: &str,
        config: ConcurrencyConfig,
    ) -> Result<(), LambdaError> {
        let func = self.get_function(name).await?;
        let now = chrono::Utc::now();
        let reserved = config.reserved_concurrent_executions.map(|v| v as i64);
        sqlx::query(
            r#"INSERT INTO function_concurrency(function_id, reserved_concurrent_executions, updated_at)
               VALUES(?, ?, ?)
               ON CONFLICT(function_id) DO UPDATE SET reserved_concurrent_executions = excluded.reserved_concurrent_executions, updated_at = excluded.updated_at"#
        )
        .bind(func.function_id)
        .bind(reserved)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?;
        // Update in-memory limiter
        self.concurrency_manager
            .set_reserved_limit(func.function_id, config.reserved_concurrent_executions);

        // Invalidate cache since concurrency config was updated
        self.cache
            .invalidate_concurrency(&func.function_id.to_string());

        Ok(())
    }

    #[instrument(skip(self))]
    pub async fn get_concurrency(&self, name: &str) -> Result<ConcurrencyConfig, LambdaError> {
        let func = self.get_function(name).await?;

        // Try cache first
        if let Some(cached_config) = self.cache.get_concurrency(&func.function_id.to_string()) {
            return Ok(cached_config);
        }

        // Cache miss - fetch from database
        let reserved: Option<i64> = sqlx::query_scalar(
            "SELECT reserved_concurrent_executions FROM function_concurrency WHERE function_id = ?",
        )
        .bind(func.function_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(LambdaError::SqlxError)?
        .flatten();

        let config = ConcurrencyConfig {
            reserved_concurrent_executions: reserved.map(|v| v as u32),
        };

        // Cache the result
        self.cache
            .set_concurrency(func.function_id.to_string(), config.clone());

        Ok(config)
    }

    #[instrument(skip(self))]
    pub async fn delete_concurrency(&self, name: &str) -> Result<(), LambdaError> {
        let func = self.get_function(name).await?;
        let _ = sqlx::query("DELETE FROM function_concurrency WHERE function_id = ?")
            .bind(func.function_id)
            .execute(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;
        self.concurrency_manager
            .set_reserved_limit(func.function_id, None);

        // Invalidate cache since concurrency config was deleted
        self.cache
            .invalidate_concurrency(&func.function_id.to_string());

        Ok(())
    }

    #[instrument(skip(self, request))]
    pub async fn invoke_function(
        &self,
        request: InvokeRequest,
    ) -> Result<InvokeResponse, LambdaError> {
        info!("Invoking function: {}", request.function_name);

        // 1) Lookup function meta from Registry. If not found → 404.
        let function = self.get_function(&request.function_name).await?;

        // 2) Acquire concurrency token (RAII guard ensures release on any exit)
        let _token_guard = self.concurrency_manager.acquire_token(&function).await?;

        // 3) Create request ID: req_id = Uuid::new_v4().to_string()
        let req_id = uuid::Uuid::new_v4().to_string();

        // 3.5) Record execution start in database (deferred async)
        let start_time = chrono::Utc::now();
        self.execution_tracker.record_execution_start(
            req_id.clone(),
            &function,
            req_id.clone(),
            start_time,
        );

        // 4) Register pending waiter: let rx = pending.register(req_id.clone())
        let rx = self.scheduler.pending().register(req_id.clone());

        // 5) Build WorkItem
        let work_item =
            WorkItem::from_invoke_request(req_id.clone(), function.clone(), request.clone());

        // 6) Ensure at least one warm container exists for this function-key (fn+rt+ver+env)
        // Important: do NOT consume availability here. Just check count to avoid
        // toggling a container to unavailable inadvertently.
        let fn_key = crate::queues::FnKey::from_work_item(&work_item);
        if self.warm_pool.container_count(&fn_key).await == 0 {
            info!(
                "No container present, creating new container for function: {}",
                function.function_name
            );

            // Build image reference
            let image_ref = format!(
                "lambda-home/{}:{}",
                function.function_name, function.code_sha256
            );

            // Build Docker image first
            let mut packaging_service =
                lambda_packaging::PackagingService::new(self.config.clone());
            packaging_service.build_image(&function, &image_ref).await?;

            // Create container: generate instance id and inject as env
            let instance_id = uuid::Uuid::new_v4().to_string();
            let mut env_vars = self.resolve_env_vars(&function).await?;
            env_vars.insert("LAMBDAH_INSTANCE_ID".to_string(), instance_id.clone());
            let container_id = self
                .invoker
                .create_container(&function, &image_ref, env_vars)
                .await?;
            self.invoker.start_container(&container_id).await?;

            // Add to warm pool
            let warm_container = crate::warm_pool::WarmContainer {
                container_id: container_id.clone(),
                instance_id: instance_id.clone(),
                function_id: function.function_id,
                image_ref: image_ref.clone(),
                created_at: std::time::Instant::now(),
                last_used: std::time::Instant::now(),
                state: crate::warm_pool::InstanceState::WarmIdle, // Ready for work
            };
            self.warm_pool
                .add_warm_container(fn_key.clone(), warm_container)
                .await;

            info!(
                "Created and started new container: {} for function: {}",
                container_id, function.function_name
            );
        } else if !self.warm_pool.has_available(&fn_key).await {
            // Prefer restarting a stopped container for this key
            if let Some(stopped_id) = self.warm_pool.get_one_stopped(&fn_key).await {
                info!(
                    "Re-starting stopped container {} for function: {}",
                    stopped_id, function.function_name
                );
                self.invoker.start_container(&stopped_id).await?;
                let _ = self
                    .warm_pool
                    .set_state_by_container_id(
                        &stopped_id,
                        crate::warm_pool::InstanceState::WarmIdle,
                    )
                    .await;
            } else {
                // All existing containers are busy; scale up by creating a new one
                info!(
                    "All containers busy for {}. Scaling up by 1.",
                    function.function_name
                );
                let image_ref = format!(
                    "lambda-home/{}:{}",
                    function.function_name, function.code_sha256
                );
                let mut packaging_service =
                    lambda_packaging::PackagingService::new(self.config.clone());
                packaging_service.build_image(&function, &image_ref).await?;

                let instance_id = uuid::Uuid::new_v4().to_string();
                let mut env_vars = self.resolve_env_vars(&function).await?;
                env_vars.insert("LAMBDAH_INSTANCE_ID".to_string(), instance_id.clone());
                let container_id = self
                    .invoker
                    .create_container(&function, &image_ref, env_vars)
                    .await?;
                self.invoker.start_container(&container_id).await?;

                let warm_container = crate::warm_pool::WarmContainer {
                    container_id: container_id.clone(),
                    instance_id: instance_id.clone(),
                    function_id: function.function_id,
                    image_ref: image_ref.clone(),
                    created_at: std::time::Instant::now(),
                    last_used: std::time::Instant::now(),
                    state: crate::warm_pool::InstanceState::WarmIdle,
                };
                self.warm_pool
                    .add_warm_container(fn_key, warm_container)
                    .await;
                info!(
                    "Scaled up with new container: {} for function: {}",
                    container_id, function.function_name
                );
            }
        }

        // 7) Enqueue: scheduler.enqueue(work_item).await
        self.scheduler
            .enqueue(work_item)
            .await
            .map_err(|e| LambdaError::InternalError {
                reason: format!("Failed to enqueue work item: {e}"),
            })?;

        // 8) Wait for result with buffer: 10 seconds for container startup and execution
        let total = tokio::time::Duration::from_secs(10);
        match tokio::time::timeout(total, rx).await {
            Ok(Ok(result)) => {
                // Success: build Lambda response
                if result.ok {
                    Ok(InvokeResponse {
                        status_code: 200,
                        payload: Some(
                            serde_json::from_slice(&result.payload)
                                .unwrap_or(serde_json::Value::Null),
                        ),
                        executed_version: result.executed_version,
                        function_error: None,
                        log_result: result.log_tail_b64,
                        headers: std::collections::HashMap::new(),
                    })
                } else {
                    // Function error: return 200 with X-Amz-Function-Error header
                    Ok(InvokeResponse {
                        status_code: 200,
                        payload: Some(
                            serde_json::from_slice(&result.payload)
                                .unwrap_or(serde_json::Value::Null),
                        ),
                        executed_version: result.executed_version,
                        function_error: result.function_error.as_ref().map(|fe| {
                            match fe.as_str() {
                                "Handled" => FunctionError::Handled,
                                _ => FunctionError::Unhandled,
                            }
                        }),
                        log_result: result.log_tail_b64,
                        headers: std::collections::HashMap::new(),
                    })
                }
            }
            Ok(Err(_canceled)) => {
                // Runtime channel dropped → 200 with X-Amz-Function-Error: InitError
                error!("Runtime channel closed for invocation: {}", req_id);

                // Record init error in execution record (deferred async)
                let end_time = chrono::Utc::now();
                self.execution_tracker
                    .record_execution_init_error(req_id.clone(), end_time);

                Ok(InvokeResponse {
                    status_code: 200,
                    payload: Some(serde_json::json!({
                        "errorMessage": "Runtime channel closed",
                        "errorType": "InitError"
                    })),
                    executed_version: Some("1".to_string()),
                    function_error: Some(FunctionError::Unhandled),
                    log_result: None,
                    headers: std::collections::HashMap::new(),
                })
            }
            Err(_elapsed) => {
                // Timeout: mark timeout and return 200 with X-Amz-Function-Error: Unhandled
                let timeout_json = serde_json::json!({
                    "errorMessage": format!("Task timed out after {} seconds", function.timeout),
                    "errorType": "TaskTimedOut"
                });
                let timeout_body = serde_json::to_vec(&timeout_json).unwrap_or_default();
                let _ =
                    self.scheduler
                        .pending()
                        .fail_if_waiting(&req_id, "Unhandled", timeout_body);

                // Record timeout in execution record (deferred async)
                let end_time = chrono::Utc::now();
                self.execution_tracker
                    .record_execution_timeout(req_id.clone(), end_time);

                Ok(InvokeResponse {
                    status_code: 200,
                    payload: Some(timeout_json),
                    executed_version: Some("1".to_string()),
                    function_error: Some(FunctionError::Unhandled),
                    log_result: None,
                    headers: std::collections::HashMap::new(),
                })
            }
        }
        // Token guard automatically releases concurrency token when dropped
    }

    #[instrument(skip(self))]
    pub async fn get_next_invocation(
        &self,
        function_name: &str,
        runtime: &str,
        version: Option<&str>,
        env_hash: Option<&str>,
    ) -> Result<RuntimeInvocation, LambdaError> {
        // Runtime Long-Poll (GET /2018-06-01/runtime/invocation/next)
        // Goal: Container pulls work; this call blocks until work is available.

        debug!(
            "Container polling for next invocation for function: {} runtime: {}",
            function_name, runtime
        );

        // 1) Pop or wait: lost-wakeup safe, keyed by fn+rt+ver+env
        let key = crate::queues::FnKey {
            function_name: function_name.to_string(),
            runtime: runtime.to_string(),
            version: version.unwrap_or("LATEST").to_string(),
            env_hash: env_hash.unwrap_or("").to_string(),
        };
        let work_item = self.scheduler.queues().pop_or_wait(&key).await?;

        // Active marking handled by runtime API using instance header

        debug!(
            "Found work item: {} for function: {}",
            work_item.request_id, work_item.function.function_name
        );

        // 3) Return JSON in AWS Lambda Runtime API format
        Ok(RuntimeInvocation {
            aws_request_id: Uuid::parse_str(&work_item.request_id).map_err(|_| {
                LambdaError::InvalidRequest {
                    reason: "Invalid request ID".to_string(),
                }
            })?,
            deadline_ms: work_item.deadline_ms,
            invoked_function_arn: format!(
                "arn:aws:lambda:us-east-1:123456789012:function:{}",
                work_item.function.function_name
            ),
            trace_id: None,
            client_context: work_item.client_context.clone(),
            cognito_identity: work_item.cognito_identity.clone(),
            payload: serde_json::from_slice(&work_item.payload).unwrap_or(serde_json::Value::Null),
        })
    }

    #[instrument(skip(self))]
    pub async fn post_response(
        &self,
        response: RuntimeResponse,
        headers: Option<std::collections::HashMap<String, String>>,
    ) -> Result<(), LambdaError> {
        let request_id = response.aws_request_id.to_string();
        info!(
            "Processing response from container for request: {}",
            request_id
        );

        // Success: POST /2018-06-01/runtime/invocation/{requestId}/response
        // Build InvocationResult::ok(payload)
        let payload = serde_json::to_vec(&response.payload).unwrap_or_default();
        let mut result = crate::pending::InvocationResult::ok(payload);

        // Optional headers: X-Amz-Executed-Version, X-Amz-Log-Result
        if let Some(headers) = headers {
            if let Some(executed_version) = headers.get("X-Amz-Executed-Version") {
                result.executed_version = Some(executed_version.clone());
            }
            if let Some(log_result) = headers.get("X-Amz-Log-Result") {
                result.log_tail_b64 = Some(log_result.clone());
            }
        }

        // pending.complete(&request_id, res) → 202 if delivered, 404 if no waiter (late / duplicate)
        let success = self.scheduler.pending().complete(&request_id, result);
        // Best-effort: mark some active container back to WarmIdle
        let _ = self.warm_pool.mark_any_active_to_idle().await;
        if success {
            // Record successful execution completion (deferred async)
            let end_time = chrono::Utc::now();
            self.execution_tracker
                .record_execution_success(request_id.clone(), end_time);

            info!("Successfully completed invocation: {}", request_id);
            Ok(())
        } else {
            error!(
                "Failed to complete invocation {}: not found in pending (late/duplicate)",
                request_id
            );
            Err(LambdaError::InvalidRequest {
                reason: "Invocation not found".to_string(),
            })
        }
    }

    #[instrument(skip(self))]
    pub async fn post_error(
        &self,
        error: RuntimeError,
        headers: Option<std::collections::HashMap<String, String>>,
    ) -> Result<(), LambdaError> {
        let request_id = error.aws_request_id.to_string();
        info!(
            "Processing error from container for request: {}",
            request_id
        );

        // Error: POST /2018-06-01/runtime/invocation/{requestId}/error
        // Build InvocationResult::err(kind, payload) where kind from header X-Amz-Function-Error or default "Unhandled"
        let error_payload = serde_json::json!({
            "errorMessage": error.error_message,
            "errorType": error.error_type,
            "stackTrace": error.stack_trace
        });
        let payload = serde_json::to_vec(&error_payload).unwrap_or_default();

        let error_kind = headers
            .as_ref()
            .and_then(|h| h.get("X-Amz-Function-Error"))
            .map(|s| s.as_str())
            .unwrap_or("Unhandled");

        let mut result = crate::pending::InvocationResult::err(error_kind, payload);

        // Optionally set X-Amz-Log-Result into res.log_tail_b64
        if let Some(headers) = headers {
            if let Some(log_result) = headers.get("X-Amz-Log-Result") {
                result.log_tail_b64 = Some(log_result.clone());
            }
        }

        // pending.complete(&request_id, res) → 202 or 404 same as above
        let success = self.scheduler.pending().complete(&request_id, result);
        // Best-effort: mark some active container back to WarmIdle
        let _ = self.warm_pool.mark_any_active_to_idle().await;
        if success {
            // Record failed execution completion (deferred async)
            let end_time = chrono::Utc::now();
            self.execution_tracker.record_execution_failure(
                request_id.clone(),
                error.error_type.clone(),
                end_time,
            );

            info!("Successfully completed error invocation: {}", request_id);
            Ok(())
        } else {
            error!(
                "Failed to complete error invocation {}: not found in pending (late/duplicate)",
                request_id
            );
            Err(LambdaError::InvalidRequest {
                reason: "Invocation not found".to_string(),
            })
        }
    }

    #[instrument(skip(self))]
    pub async fn post_init_error(&self, error: InitError) -> Result<(), LambdaError> {
        // TODO: Implement posting init error from containers
        Ok(())
    }

    // Public helpers for runtime API to mark instance state
    pub async fn mark_instance_active_by_id(
        &self,
        instance_id: &str,
    ) -> Option<(crate::queues::FnKey, String)> {
        self.warm_pool.mark_active_by_instance(instance_id).await
    }
    pub async fn mark_instance_idle_by_id(
        &self,
        instance_id: &str,
    ) -> Option<(crate::queues::FnKey, String)> {
        self.warm_pool.mark_idle_by_instance(instance_id).await
    }

    // Helper methods

    fn is_valid_function_name(&self, name: &str) -> bool {
        // AWS Lambda function name validation rules
        !name.is_empty()
            && name.len() <= 64
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    fn is_valid_runtime(&self, runtime: &str) -> bool {
        matches!(runtime, "nodejs18.x" | "nodejs22.x" | "python3.11" | "rust")
    }

    async fn function_exists(&self, name: &str) -> Result<bool, LambdaError> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM functions WHERE function_name = ?")
                .bind(name)
                .fetch_one(&self.pool)
                .await
                .map_err(LambdaError::SqlxError)?;

        Ok(count > 0)
    }

    fn row_to_function(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Function, LambdaError> {
        let environment: HashMap<String, String> = serde_json::from_str(
            row.try_get::<String, _>("environment")
                .map_err(LambdaError::SqlxError)?
                .as_str(),
        )
        .unwrap_or_default();

        let state: FunctionState = serde_json::from_str(
            row.try_get::<String, _>("state")
                .map_err(LambdaError::SqlxError)?
                .as_str(),
        )
        .unwrap_or(FunctionState::Pending);

        Ok(Function {
            function_id: row.try_get("function_id").map_err(LambdaError::SqlxError)?,
            function_name: row
                .try_get("function_name")
                .map_err(LambdaError::SqlxError)?,
            runtime: row.try_get("runtime").map_err(LambdaError::SqlxError)?,
            role: row.try_get("role").map_err(LambdaError::SqlxError)?,
            handler: row.try_get("handler").map_err(LambdaError::SqlxError)?,
            code_sha256: row.try_get("code_sha256").map_err(LambdaError::SqlxError)?,
            description: row.try_get("description").map_err(LambdaError::SqlxError)?,
            timeout: row
                .try_get::<i64, _>("timeout")
                .map_err(LambdaError::SqlxError)? as u64,
            memory_size: row
                .try_get::<i64, _>("memory_size")
                .map_err(LambdaError::SqlxError)? as u64,
            environment,
            last_modified: row
                .try_get("last_modified")
                .map_err(LambdaError::SqlxError)?,
            code_size: row
                .try_get::<i64, _>("code_size")
                .map_err(LambdaError::SqlxError)? as u64,
            version: row.try_get("version").map_err(LambdaError::SqlxError)?,
            state,
            state_reason: row
                .try_get("state_reason")
                .map_err(LambdaError::SqlxError)?,
            state_reason_code: row
                .try_get("state_reason_code")
                .map_err(LambdaError::SqlxError)?,
        })
    }

    fn row_to_version(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Version, LambdaError> {
        Ok(Version {
            version_id: row.try_get("version_id").map_err(LambdaError::SqlxError)?,
            function_id: row.try_get("function_id").map_err(LambdaError::SqlxError)?,
            version: row.try_get("version").map_err(LambdaError::SqlxError)?,
            description: row.try_get("description").map_err(LambdaError::SqlxError)?,
            code_sha256: row.try_get("code_sha256").map_err(LambdaError::SqlxError)?,
            last_modified: row
                .try_get("last_modified")
                .map_err(LambdaError::SqlxError)?,
            code_size: row
                .try_get::<i64, _>("code_size")
                .map_err(LambdaError::SqlxError)? as u64,
        })
    }

    fn row_to_alias(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Alias, LambdaError> {
        let routing_config: Option<RoutingConfig> = serde_json::from_str(
            row.try_get::<String, _>("routing_config")
                .map_err(LambdaError::SqlxError)?
                .as_str(),
        )
        .ok();

        Ok(Alias {
            alias_id: row.try_get("alias_id").map_err(LambdaError::SqlxError)?,
            function_id: row.try_get("function_id").map_err(LambdaError::SqlxError)?,
            name: row.try_get("name").map_err(LambdaError::SqlxError)?,
            function_version: row
                .try_get("function_version")
                .map_err(LambdaError::SqlxError)?,
            description: row.try_get("description").map_err(LambdaError::SqlxError)?,
            routing_config,
            revision_id: row.try_get("revision_id").map_err(LambdaError::SqlxError)?,
            last_modified: row
                .try_get("last_modified")
                .map_err(LambdaError::SqlxError)?,
        })
    }
}

fn normalize_path(p: &str) -> String {
    let mut s = if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{p}")
    };
    if s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    s
}

impl ControlPlane {
    // Resolve environment variables, replacing secret references with actual values.
    // Secret reference format: "SECRET_REF:<name>"
    pub async fn resolve_env_vars(
        &self,
        function: &Function,
    ) -> Result<HashMap<String, String>, LambdaError> {
        // Try cache first
        if let Some(cached_env_vars) = self.cache.get_env_vars(&function.function_id.to_string()) {
            return Ok(cached_env_vars);
        }

        // Cache miss - resolve from function and secrets
        let mut out = function.environment.clone();
        // Only fetch secrets when needed to avoid extra queries
        for (_k, v) in out.clone().iter() {
            if let Some(name) = v.strip_prefix("SECRET_REF:") {
                // Lookup secret value
                if let Some(val) = self.get_secret_value(name).await? {
                    out.insert(_k.clone(), val);
                } else {
                    // If missing, leave as-is (keeps reference) to help debugging
                }
            }
        }

        // Cache the result
        self.cache
            .set_env_vars(function.function_id.to_string(), out.clone());

        Ok(out)
    }

    // Secrets management helpers
    pub async fn create_secret(&self, name: &str, value: &str) -> Result<(), LambdaError> {
        let now = chrono::Utc::now();
        // Store base64-encoded; note: this is obfuscation, not strong encryption.
        let encoded =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, value.as_bytes());
        sqlx::query("INSERT OR REPLACE INTO secrets(name, value, created_at) VALUES(?, ?, ?)")
            .bind(name)
            .bind(encoded)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;
        // Drain any functions referencing this secret so next invokes pick up the new value
        self.drain_functions_referencing_secret(name).await?;
        Ok(())
    }

    pub async fn list_secrets(
        &self,
    ) -> Result<Vec<(String, chrono::DateTime<chrono::Utc>)>, LambdaError> {
        let rows = sqlx::query("SELECT name, created_at FROM secrets ORDER BY name")
            .fetch_all(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;
        let mut v = Vec::with_capacity(rows.len());
        for r in rows.iter() {
            let name: String = r.try_get("name").map_err(LambdaError::SqlxError)?;
            let created: chrono::DateTime<chrono::Utc> =
                r.try_get("created_at").map_err(LambdaError::SqlxError)?;
            v.push((name, created));
        }
        Ok(v)
    }

    pub async fn delete_secret(&self, name: &str) -> Result<(), LambdaError> {
        let _ = sqlx::query("DELETE FROM secrets WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;

        // Invalidate cache since secret was deleted
        self.cache.invalidate_secret(name);

        // Drain any functions referencing this secret so next invokes fail/reflect missing secret
        self.drain_functions_referencing_secret(name).await?;
        Ok(())
    }

    pub async fn get_secret_value(&self, name: &str) -> Result<Option<String>, LambdaError> {
        // Try cache first
        if let Some(cached_value) = self.cache.get_secret(name) {
            return Ok(Some(cached_value));
        }

        // Cache miss - fetch from database
        let val: Option<String> = sqlx::query_scalar("SELECT value FROM secrets WHERE name = ?")
            .bind(name)
            .fetch_optional(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;

        if let Some(enc) = val {
            if let Ok(bytes) =
                base64::Engine::decode(&base64::engine::general_purpose::STANDARD, enc)
            {
                if let Ok(s) = String::from_utf8(bytes) {
                    // Cache the result
                    self.cache.set_secret(name.to_string(), s.clone());
                    return Ok(Some(s));
                }
            }
            return Ok(None);
        }
        Ok(None)
    }

    async fn drain_functions_referencing_secret(&self, name: &str) -> Result<(), LambdaError> {
        // Find functions whose environment JSON contains this secret reference
        let pattern = format!("%SECRET_REF:{name}%");
        let rows = sqlx::query("SELECT function_id FROM functions WHERE environment LIKE ?")
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await
            .map_err(LambdaError::SqlxError)?;
        for row in rows {
            let fid: uuid::Uuid = row.try_get("function_id").map_err(LambdaError::SqlxError)?;
            let ids = self.warm_pool.drain_by_function_id(fid).await;
            for id in ids {
                let _ = self.invoker.remove_container(&id).await;
            }
        }
        Ok(())
    }

    /// Get comprehensive Docker and cache statistics
    #[instrument(skip(self))]
    pub async fn get_docker_stats(&self) -> Result<DockerStats, LambdaError> {
        // Get Docker stats from invoker
        let mut docker_stats =
            self.invoker
                .get_docker_stats()
                .await
                .map_err(|e| LambdaError::DockerError {
                    message: format!("Failed to get Docker stats: {e}"),
                })?;

        // Get cache stats
        let cache_stats = self.cache.get_stats();
        let cache_sizes = self.cache.get_sizes();

        // Convert cache stats to our model
        let cache_stats_model = CacheStats {
            functions: CacheTypeStats {
                hits: cache_stats.get("functions").map(|s| s.hits).unwrap_or(0),
                misses: cache_stats.get("functions").map(|s| s.misses).unwrap_or(0),
                evictions: cache_stats
                    .get("functions")
                    .map(|s| s.evictions)
                    .unwrap_or(0),
                invalidations: cache_stats
                    .get("functions")
                    .map(|s| s.invalidations)
                    .unwrap_or(0),
                hit_rate: cache_stats
                    .get("functions")
                    .map(|s| s.hit_rate())
                    .unwrap_or(0.0),
                size: cache_sizes.get("functions").copied().unwrap_or(0),
            },
            concurrency: CacheTypeStats {
                hits: cache_stats.get("concurrency").map(|s| s.hits).unwrap_or(0),
                misses: cache_stats
                    .get("concurrency")
                    .map(|s| s.misses)
                    .unwrap_or(0),
                evictions: cache_stats
                    .get("concurrency")
                    .map(|s| s.evictions)
                    .unwrap_or(0),
                invalidations: cache_stats
                    .get("concurrency")
                    .map(|s| s.invalidations)
                    .unwrap_or(0),
                hit_rate: cache_stats
                    .get("concurrency")
                    .map(|s| s.hit_rate())
                    .unwrap_or(0.0),
                size: cache_sizes.get("concurrency").copied().unwrap_or(0),
            },
            env_vars: CacheTypeStats {
                hits: cache_stats.get("env_vars").map(|s| s.hits).unwrap_or(0),
                misses: cache_stats.get("env_vars").map(|s| s.misses).unwrap_or(0),
                evictions: cache_stats
                    .get("env_vars")
                    .map(|s| s.evictions)
                    .unwrap_or(0),
                invalidations: cache_stats
                    .get("env_vars")
                    .map(|s| s.invalidations)
                    .unwrap_or(0),
                hit_rate: cache_stats
                    .get("env_vars")
                    .map(|s| s.hit_rate())
                    .unwrap_or(0.0),
                size: cache_sizes.get("env_vars").copied().unwrap_or(0),
            },
            secrets: CacheTypeStats {
                hits: cache_stats.get("secrets").map(|s| s.hits).unwrap_or(0),
                misses: cache_stats.get("secrets").map(|s| s.misses).unwrap_or(0),
                evictions: cache_stats.get("secrets").map(|s| s.evictions).unwrap_or(0),
                invalidations: cache_stats
                    .get("secrets")
                    .map(|s| s.invalidations)
                    .unwrap_or(0),
                hit_rate: cache_stats
                    .get("secrets")
                    .map(|s| s.hit_rate())
                    .unwrap_or(0.0),
                size: cache_sizes.get("secrets").copied().unwrap_or(0),
            },
        };

        // Add cache stats to Docker stats
        docker_stats.cache_stats = Some(cache_stats_model);

        Ok(docker_stats)
    }

    /// Get Lambda service statistics
    #[instrument(skip(self))]
    pub async fn get_lambda_service_stats(
        &self,
    ) -> anyhow::Result<lambda_models::LambdaServiceStats> {
        // Get function statistics from database
        let function_rows =
            sqlx::query("SELECT state, memory_size FROM functions ORDER BY last_modified DESC")
                .fetch_all(&self.pool)
                .await?;

        // Get warm pool statistics
        let total_containers = self.warm_pool.total_container_count().await;
        let active_containers = 0; // TODO: Implement active container counting
        let idle_containers = total_containers - active_containers;

        // Calculate totals
        let total_functions = function_rows.len() as u64;
        let active_functions = function_rows
            .iter()
            .filter(|row| row.get::<String, _>("state") == "\"Active\"")
            .count() as u64;
        let stopped_functions = function_rows
            .iter()
            .filter(|row| row.get::<String, _>("state") == "\"Inactive\"")
            .count() as u64;
        let failed_functions = function_rows
            .iter()
            .filter(|row| row.get::<String, _>("state") == "\"Failed\"")
            .count() as u64;

        // Calculate total memory and CPU allocation
        let total_memory_mb: u64 = function_rows
            .iter()
            .filter(|row| row.get::<String, _>("state") == "\"Active\"")
            .map(|row| row.get::<i64, _>("memory_size") as u64)
            .sum();

        let total_cpu_cores: u64 = function_rows
            .iter()
            .filter(|row| row.get::<String, _>("state") == "\"Active\"")
            .map(|row| (row.get::<i64, _>("memory_size") as f64 / 1024.0).ceil() as u64) // Rough estimate: 1 CPU core per 1GB memory
            .sum();

        // Get execution statistics
        let execution_stats = sqlx::query(
            "SELECT 
                COUNT(*) as total_invocations,
                COUNT(CASE WHEN error_type IS NULL THEN 1 END) as successful_invocations,
                COUNT(CASE WHEN error_type IS NOT NULL THEN 1 END) as failed_invocations,
                AVG(CAST(duration_ms AS REAL)) as avg_duration_ms,
                MAX(CAST(duration_ms AS REAL)) as max_duration_ms,
                MIN(CAST(duration_ms AS REAL)) as min_duration_ms
            FROM executions 
            WHERE start_time > datetime('now', '-24 hours')",
        )
        .fetch_one(&self.pool)
        .await?;

        let stats = lambda_models::LambdaServiceStats {
            total_functions,
            active_functions,
            stopped_functions,
            failed_functions,
            total_memory_mb,
            total_cpu_cores,
            warm_containers: total_containers as u64,
            active_containers: active_containers as u64,
            idle_containers: idle_containers as u64,
            total_invocations_24h: execution_stats.get::<i64, _>("total_invocations") as u64,
            successful_invocations_24h: execution_stats.get::<i64, _>("successful_invocations")
                as u64,
            failed_invocations_24h: execution_stats.get::<i64, _>("failed_invocations") as u64,
            avg_duration_ms: execution_stats
                .get::<Option<f64>, _>("avg_duration_ms")
                .unwrap_or(0.0),
            max_duration_ms: execution_stats
                .get::<Option<f64>, _>("max_duration_ms")
                .unwrap_or(0.0),
            min_duration_ms: execution_stats
                .get::<Option<f64>, _>("min_duration_ms")
                .unwrap_or(0.0),
        };

        Ok(stats)
    }
}
