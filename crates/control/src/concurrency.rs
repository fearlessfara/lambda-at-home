use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::info;
use lambda_models::{Function, LambdaError};

#[derive(Clone)]
pub struct Concurrency {
    sem: Arc<Semaphore>,
}

impl Concurrency {
    pub fn new(limit: usize) -> Self { 
        Self { sem: Arc::new(Semaphore::new(limit)) } 
    }
    
    /// Acquire a concurrency token with RAII guard
    /// The token is automatically released when the guard is dropped
    pub async fn acquire(&self) -> anyhow::Result<TokenGuard> {
        let permit = self.sem.clone().acquire_owned().await?;
        info!("Acquired concurrency token, {} remaining", self.sem.available_permits());
        Ok(TokenGuard { _permit: permit })
    }
    
    /// Get the number of available permits
    pub fn available_permits(&self) -> usize {
        self.sem.available_permits()
    }
}

/// RAII guard that holds a concurrency token
/// The token is automatically released when this guard is dropped
pub struct TokenGuard {
    _permit: tokio::sync::OwnedSemaphorePermit, // keeps the token until drop
}

impl Drop for TokenGuard {
    fn drop(&mut self) {
        // Token is automatically released when the permit is dropped
    }
}

/// Concurrency manager that provides per-function or global concurrency control
#[derive(Clone)]
pub struct ConcurrencyManager {
    global: Concurrency,
}

impl ConcurrencyManager {
    pub fn new() -> Self {
        Self {
            global: Concurrency::new(256), // Default global limit
        }
    }
    
    pub fn with_max_concurrency(limit: usize) -> Self {
        Self {
            global: Concurrency::new(limit),
        }
    }
    
    /// Acquire a concurrency token for a function
    /// Currently uses global concurrency, but can be extended for per-function limits
    pub async fn acquire_token(&self, _function: &Function) -> Result<TokenGuard, LambdaError> {
        self.global.acquire().await.map_err(|e| LambdaError::InternalError { 
            reason: format!("Failed to acquire concurrency token: {}", e) 
        })
    }
    
    /// Try to acquire a concurrency token without blocking
    /// Returns an error if no tokens are available
    pub fn try_acquire_token(&self, _function: &Function) -> Result<TokenGuard, LambdaError> {
        self.global.sem.clone().try_acquire_owned().map(|permit| TokenGuard { _permit: permit })
            .map_err(|_| LambdaError::InternalError { 
                reason: "No concurrency tokens available".to_string() 
            })
    }
}