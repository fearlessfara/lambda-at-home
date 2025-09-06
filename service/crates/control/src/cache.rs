use dashmap::DashMap;
use lambda_models::{Function, ConcurrencyConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Cache entry with TTL support
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    value: T,
    created_at: Instant,
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            created_at: Instant::now(),
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub invalidations: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Comprehensive function metadata cache
#[derive(Clone)]
pub struct FunctionCache {
    // Core caches
    functions: Arc<DashMap<String, CacheEntry<Arc<Function>>>>,
    concurrency: Arc<DashMap<String, CacheEntry<ConcurrencyConfig>>>,
    env_vars: Arc<DashMap<String, CacheEntry<HashMap<String, String>>>>,
    secrets: Arc<DashMap<String, CacheEntry<String>>>,
    
    // Configuration
    default_ttl: Duration,
    max_size: usize,
    
    // Statistics
    stats: Arc<DashMap<String, CacheStats>>,
}

impl FunctionCache {
    pub fn new(default_ttl: Duration, max_size: usize) -> Self {
        Self {
            functions: Arc::new(DashMap::new()),
            concurrency: Arc::new(DashMap::new()),
            env_vars: Arc::new(DashMap::new()),
            secrets: Arc::new(DashMap::new()),
            default_ttl,
            max_size,
            stats: Arc::new(DashMap::new()),
        }
    }

    /// Get function metadata with cache
    pub fn get_function(&self, name: &str) -> Option<Arc<Function>> {
        if let Some(entry) = self.functions.get(name) {
            if entry.is_expired() {
                debug!("Function cache miss (expired): {}", name);
                self.increment_miss("functions");
                drop(entry);
                self.functions.remove(name);
                None
            } else {
                debug!("Function cache hit: {}", name);
                self.increment_hit("functions");
                Some(entry.value.clone())
            }
        } else {
            debug!("Function cache miss (not found): {}", name);
            self.increment_miss("functions");
            None
        }
    }

    /// Set function metadata in cache
    pub fn set_function(&self, name: String, function: Function) {
        self.evict_if_needed("functions");
        let entry = CacheEntry::new(Arc::new(function), self.default_ttl);
        self.functions.insert(name, entry);
        debug!("Function cached");
    }

    /// Get concurrency config with cache
    pub fn get_concurrency(&self, function_id: &str) -> Option<ConcurrencyConfig> {
        if let Some(entry) = self.concurrency.get(function_id) {
            if entry.is_expired() {
                debug!("Concurrency cache miss (expired): {}", function_id);
                self.increment_miss("concurrency");
                drop(entry);
                self.concurrency.remove(function_id);
                None
            } else {
                debug!("Concurrency cache hit: {}", function_id);
                self.increment_hit("concurrency");
                Some(entry.value.clone())
            }
        } else {
            debug!("Concurrency cache miss (not found): {}", function_id);
            self.increment_miss("concurrency");
            None
        }
    }

    /// Set concurrency config in cache
    pub fn set_concurrency(&self, function_id: String, config: ConcurrencyConfig) {
        self.evict_if_needed("concurrency");
        let entry = CacheEntry::new(config, self.default_ttl);
        self.concurrency.insert(function_id, entry);
        debug!("Concurrency config cached");
    }

    /// Get environment variables with cache
    pub fn get_env_vars(&self, function_id: &str) -> Option<HashMap<String, String>> {
        if let Some(entry) = self.env_vars.get(function_id) {
            if entry.is_expired() {
                debug!("Env vars cache miss (expired): {}", function_id);
                self.increment_miss("env_vars");
                drop(entry);
                self.env_vars.remove(function_id);
                None
            } else {
                debug!("Env vars cache hit: {}", function_id);
                self.increment_hit("env_vars");
                Some(entry.value.clone())
            }
        } else {
            debug!("Env vars cache miss (not found): {}", function_id);
            self.increment_miss("env_vars");
            None
        }
    }

    /// Set environment variables in cache
    pub fn set_env_vars(&self, function_id: String, env_vars: HashMap<String, String>) {
        self.evict_if_needed("env_vars");
        let entry = CacheEntry::new(env_vars, self.default_ttl);
        self.env_vars.insert(function_id, entry);
        debug!("Environment variables cached");
    }

    /// Get secret value with cache
    pub fn get_secret(&self, name: &str) -> Option<String> {
        if let Some(entry) = self.secrets.get(name) {
            if entry.is_expired() {
                debug!("Secret cache miss (expired): {}", name);
                self.increment_miss("secrets");
                drop(entry);
                self.secrets.remove(name);
                None
            } else {
                debug!("Secret cache hit: {}", name);
                self.increment_hit("secrets");
                Some(entry.value.clone())
            }
        } else {
            debug!("Secret cache miss (not found): {}", name);
            self.increment_miss("secrets");
            None
        }
    }

    /// Set secret value in cache
    pub fn set_secret(&self, name: String, value: String) {
        self.evict_if_needed("secrets");
        let entry = CacheEntry::new(value, self.default_ttl);
        self.secrets.insert(name, entry);
        debug!("Secret cached");
    }

    /// Invalidate function cache (called on updates/deletes)
    pub fn invalidate_function(&self, name: &str) {
        if self.functions.remove(name).is_some() {
            debug!("Function cache invalidated: {}", name);
            self.increment_invalidation("functions");
        }
    }

    /// Invalidate concurrency cache
    pub fn invalidate_concurrency(&self, function_id: &str) {
        if self.concurrency.remove(function_id).is_some() {
            debug!("Concurrency cache invalidated: {}", function_id);
            self.increment_invalidation("concurrency");
        }
    }

    /// Invalidate environment variables cache
    pub fn invalidate_env_vars(&self, function_id: &str) {
        if self.env_vars.remove(function_id).is_some() {
            debug!("Environment variables cache invalidated: {}", function_id);
            self.increment_invalidation("env_vars");
        }
    }

    /// Invalidate secret cache
    pub fn invalidate_secret(&self, name: &str) {
        if self.secrets.remove(name).is_some() {
            debug!("Secret cache invalidated: {}", name);
            self.increment_invalidation("secrets");
        }
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.functions.clear();
        self.concurrency.clear();
        self.env_vars.clear();
        self.secrets.clear();
        info!("All caches cleared");
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&self) -> usize {
        let mut cleaned = 0;
        
        // Clean functions
        self.functions.retain(|_, entry| {
            if entry.is_expired() {
                cleaned += 1;
                false
            } else {
                true
            }
        });

        // Clean concurrency
        self.concurrency.retain(|_, entry| {
            if entry.is_expired() {
                cleaned += 1;
                false
            } else {
                true
            }
        });

        // Clean env vars
        self.env_vars.retain(|_, entry| {
            if entry.is_expired() {
                cleaned += 1;
                false
            } else {
                true
            }
        });

        // Clean secrets
        self.secrets.retain(|_, entry| {
            if entry.is_expired() {
                cleaned += 1;
                false
            } else {
                true
            }
        });

        if cleaned > 0 {
            debug!("Cleaned up {} expired cache entries", cleaned);
        }
        cleaned
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> HashMap<String, CacheStats> {
        let mut stats = HashMap::new();
        for entry in self.stats.iter() {
            stats.insert(entry.key().clone(), entry.value().clone());
        }
        stats
    }

    /// Get cache sizes
    pub fn get_sizes(&self) -> HashMap<String, usize> {
        let mut sizes = HashMap::new();
        sizes.insert("functions".to_string(), self.functions.len());
        sizes.insert("concurrency".to_string(), self.concurrency.len());
        sizes.insert("env_vars".to_string(), self.env_vars.len());
        sizes.insert("secrets".to_string(), self.secrets.len());
        sizes
    }

    // Private helper methods
    fn increment_hit(&self, cache_type: &str) {
        self.stats.entry(cache_type.to_string()).or_insert_with(CacheStats::default).hits += 1;
    }

    fn increment_miss(&self, cache_type: &str) {
        self.stats.entry(cache_type.to_string()).or_insert_with(CacheStats::default).misses += 1;
    }

    fn increment_invalidation(&self, cache_type: &str) {
        self.stats.entry(cache_type.to_string()).or_insert_with(CacheStats::default).invalidations += 1;
    }

    fn evict_if_needed(&self, cache_type: &str) {
        let current_size = match cache_type {
            "functions" => self.functions.len(),
            "concurrency" => self.concurrency.len(),
            "env_vars" => self.env_vars.len(),
            "secrets" => self.secrets.len(),
            _ => return,
        };

        if current_size >= self.max_size {
            warn!("Cache size limit reached for {}, evicting oldest entries", cache_type);
            self.evict_oldest(cache_type);
        }
    }

    fn evict_oldest(&self, cache_type: &str) {
        // Simple LRU eviction - remove 10% of entries
        let evict_count = (self.max_size / 10).max(1);
        
        match cache_type {
            "functions" => {
                let mut entries: Vec<_> = self.functions.iter().collect();
                entries.sort_by_key(|entry| entry.created_at);
                for entry in entries.into_iter().take(evict_count) {
                    self.functions.remove(entry.key());
                }
            }
            "concurrency" => {
                let mut entries: Vec<_> = self.concurrency.iter().collect();
                entries.sort_by_key(|entry| entry.created_at);
                for entry in entries.into_iter().take(evict_count) {
                    self.concurrency.remove(entry.key());
                }
            }
            "env_vars" => {
                let mut entries: Vec<_> = self.env_vars.iter().collect();
                entries.sort_by_key(|entry| entry.created_at);
                for entry in entries.into_iter().take(evict_count) {
                    self.env_vars.remove(entry.key());
                }
            }
            "secrets" => {
                let mut entries: Vec<_> = self.secrets.iter().collect();
                entries.sort_by_key(|entry| entry.created_at);
                for entry in entries.into_iter().take(evict_count) {
                    self.secrets.remove(entry.key());
                }
            }
            _ => {}
        }
        
        self.stats.entry(cache_type.to_string()).or_insert_with(CacheStats::default).evictions += evict_count as u64;
    }
}

impl Default for FunctionCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(300), 1000) // 5 minutes TTL, 1000 entries max
    }
}
