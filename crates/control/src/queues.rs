use std::collections::VecDeque;
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::Notify;

use lambda_models::LambdaError;
use crate::work_item::WorkItem;
use tracing::info;


// No need for FnKey wrapper - just use function name directly

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
    inner: Arc<DashMap<String, PerFn>>,
}

impl Queues {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }
    
    pub fn push(&self, work_item: WorkItem) -> Result<(), LambdaError> {
        let function_name = work_item.function.function_name.clone();
        info!(
            "Pushing work item {} to queue for function: {}",
            work_item.request_id, function_name
        );

        // Insert/enqueue under the entry guard, but clone Notify and drop guard before awaiting/notify
        let notify = {
            let mut per_fn = self.inner.entry(function_name.clone()).or_insert_with(PerFn::new);
            per_fn.queue.push_back(work_item);
            per_fn.notify.clone()
        };

        // Notify without holding the map guard to avoid lock contention
        notify.notify_one();

        info!("Notified waiting containers for function: {}", function_name);
        Ok(())
    }
    
    pub fn get_available_work(&self, function_name: &str) -> Vec<WorkItem> {
        if let Some(entry) = self.inner.get(function_name) {
            entry.queue.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
    
    pub async fn pop_or_wait(&self, function_name: &str) -> Result<WorkItem, LambdaError> {
        info!(
            "Container requesting work for function: {}",
            function_name
        );

        loop {
            // Fast path: try to dequeue if the per-fn queue exists and has items
            if let Some(mut entry) = self.inner.get_mut(function_name) {
                if let Some(work_item) = entry.queue.pop_front() {
                    info!("Dequeued work item: {} for function: {}", work_item.request_id, function_name);
                    return Ok(work_item);
                }

                // Prepare to wait: capture Notify and drop guard before awaiting
                let notify = entry.notify.clone();
                drop(entry);

                // Register listener BEFORE re-check to avoid lost wakeups
                let notified = notify.notified();

                // Re-check after listener registration; if an item arrived in the gap, consume it
                if let Some(mut entry2) = self.inner.get_mut(function_name) {
                    if let Some(work_item) = entry2.queue.pop_front() {
                        info!("Dequeued work item after re-check: {} for function: {}", work_item.request_id, function_name);
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
                let entry = self.inner.entry(function_name.to_string()).or_insert_with(PerFn::new);
                entry.notify.clone()
            };
            // Register listener first
            let notified = notify.notified();
            // Re-check in case a push landed between creating/reading notify and registering
            if let Some(mut entry2) = self.inner.get_mut(function_name) {
                if let Some(work_item) = entry2.queue.pop_front() {
                    info!("Dequeued work item after re-check (new-queue path): {} for function: {}",
                          work_item.request_id, function_name);
                    return Ok(work_item);
                }
            }
            // Actually wait
            notified.await;
        }
    }
    
    pub fn queue_size(&self, function_name: &str) -> usize {
        self.inner.get(function_name)
            .map(|per_fn| per_fn.queue.len())
            .unwrap_or(0)
    }
    
    pub fn total_queued(&self) -> usize {
        self.inner.iter()
            .map(|entry| entry.queue.len())
            .sum()
    }
    
    pub fn pop_work_item(&self, function_name: &str) -> Option<WorkItem> {
        if let Some(mut entry) = self.inner.get_mut(function_name) {
            entry.queue.pop_front()
        } else {
            None
        }
    }
}
