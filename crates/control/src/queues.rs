use std::collections::VecDeque;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::Notify;

use lambda_models::LambdaError;
use crate::work_item::WorkItem;
use tracing::info;
use sha2::{Sha256, Digest};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FnKey {
    pub function_name: String,
    pub runtime: String,
    pub version: String,
    pub env_hash: String,
}

impl FnKey {
    pub fn from_work_item(work_item: &WorkItem) -> Self {
        // Build a stable representation of the environment by recursively sorting object keys
        fn canonicalize(v: &Value) -> Value {
            match v {
                Value::Object(map) => {
                    let mut keys: Vec<&String> = map.keys().collect();
                    keys.sort();
                    let mut obj = serde_json::Map::with_capacity(keys.len());
                    for k in keys {
                        obj.insert(k.clone(), canonicalize(&map[k]));
                    }
                    Value::Object(obj)
                }
                Value::Array(arr) => {
                    // Arrays remain in the given order
                    Value::Array(arr.iter().map(canonicalize).collect())
                }
                _ => v.clone(),
            }
        }

        let env_value = serde_json::to_value(&work_item.function.environment).unwrap_or(Value::Null);
        let stable_env = canonicalize(&env_value);
        let stable_bytes = serde_json::to_vec(&stable_env).unwrap_or_default();

        let mut hasher = Sha256::new();
        hasher.update(&stable_bytes);
        let env_hash = format!("{:x}", hasher.finalize());

        Self {
            function_name: work_item.function.function_name.clone(),
            runtime: work_item.function.runtime.clone(),
            // Prefer explicit version from the work item if present; otherwise use "LATEST"
            version: work_item
                .function
                .version
                .clone()
                .unwrap_or_else(|| "LATEST".to_string()),
            env_hash,
        }
    }
}

#[derive(Debug)]
struct PerFn {
    queue: VecDeque<WorkItem>,
    notify: Arc<Notify>,
}

impl PerFn {
    fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            notify: Arc::new(Notify::new()),
        }
    }
}

#[derive(Clone)]
pub struct Queues {
    inner: Arc<DashMap<FnKey, PerFn>>,
}

impl Queues {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }
    
    pub fn push(&self, work_item: WorkItem) -> Result<(), LambdaError> {
        let key = FnKey::from_work_item(&work_item);
        info!(
            "Pushing work item {} to queue for function: {} runtime: {} version: {} env_hash: {}",
            work_item.request_id, key.function_name, key.runtime, key.version, key.env_hash
        );

        // Insert/enqueue under the entry guard, but clone Notify and drop guard before awaiting/notify
        let notify = {
            let mut per_fn = self.inner.entry(key.clone()).or_insert_with(PerFn::new);
            per_fn.queue.push_back(work_item);
            per_fn.notify.clone()
        };

        // Notify without holding the map guard to avoid lock contention
        notify.notify_one();

        info!("Notified waiting containers for function: {}", key.function_name);
        Ok(())
    }
    
    pub async fn pop_or_wait(&self, key: &FnKey) -> Result<WorkItem, LambdaError> {
        info!(
            "Container requesting work for function: {} runtime: {} version: {} env_hash: {}",
            key.function_name, key.runtime, key.version, key.env_hash
        );

        loop {
            // Fast path: try to dequeue if the per-fn queue exists and has items
            if let Some(mut entry) = self.inner.get_mut(key) {
                if let Some(work_item) = entry.queue.pop_front() {
                    info!("Dequeued work item: {} for function: {}", work_item.request_id, key.function_name);
                    return Ok(work_item);
                }

                // Prepare to wait: capture Notify and drop guard before awaiting
                let notify = entry.notify.clone();
                drop(entry);

                // Register listener BEFORE re-check to avoid lost wakeups
                let notified = notify.notified();

                // Re-check after listener registration; if an item arrived in the gap, consume it
                if let Some(mut entry2) = self.inner.get_mut(key) {
                    if let Some(work_item) = entry2.queue.pop_front() {
                        info!("Dequeued work item after re-check: {} for function: {}", work_item.request_id, key.function_name);
                        return Ok(work_item);
                    }
                }

                // Actually wait for the next notification
                notified.await;
                // and loop to try again
                continue;
            }

            // No queue yet: create an empty one and wait for first push
            let notify = {
                let entry = self.inner.entry(key.clone()).or_insert_with(PerFn::new);
                entry.notify.clone()
            };

            let notified = notify.notified();
            // Nothing to re-check yet (queue was empty/new); wait for first item then loop
            notified.await;
        }
    }
    
    pub fn queue_size(&self, key: &FnKey) -> usize {
        self.inner.get(key)
            .map(|per_fn| per_fn.queue.len())
            .unwrap_or(0)
    }
    
    pub fn total_queued(&self) -> usize {
        self.inner.iter()
            .map(|entry| entry.queue.len())
            .sum()
    }
}
