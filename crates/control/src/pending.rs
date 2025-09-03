use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::oneshot;
use tracing::{info, warn};

#[derive(Clone, Debug)]
pub struct InvocationResult {
    pub ok: bool,
    pub payload: Vec<u8>,
    pub log_tail_b64: Option<String>,
    pub executed_version: Option<String>,
    pub function_error: Option<String>, // "Handled" | "Unhandled"
}

impl InvocationResult {
    pub fn ok(payload: Vec<u8>) -> Self {
        Self { 
            ok: true, 
            payload, 
            log_tail_b64: None, 
            executed_version: None, 
            function_error: None 
        }
    }
    
    pub fn err(kind: &str, payload: Vec<u8>) -> Self {
        Self { 
            ok: false, 
            payload, 
            log_tail_b64: None, 
            executed_version: None, 
            function_error: Some(kind.to_string()) 
        }
    }
}

#[derive(Clone)]
pub struct Pending {
    inner: Arc<DashMap<String, oneshot::Sender<InvocationResult>>>,
}

impl Pending {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
        }
    }
    
    /// Register a pending waiter for a request ID
    /// Returns the receiver that will be notified when the result is available
    pub fn register(&self, req_id: String) -> oneshot::Receiver<InvocationResult> {
        let (tx, rx) = oneshot::channel();
        self.inner.insert(req_id.clone(), tx);
        info!("Registered pending waiter for request: {}", req_id);
        rx
    }
    
    /// Complete a pending invocation with a result
    /// Returns true if the request was found and completed, false if not found (late/duplicate)
    pub fn complete(&self, req_id: &str, res: InvocationResult) -> bool {
        if let Some((_, tx)) = self.inner.remove(req_id) {
            let _ = tx.send(res);
            info!("Completed pending invocation: {}", req_id);
            true
        } else {
            warn!("Attempted to complete unknown request: {}", req_id);
            false
        }
    }
    
    /// Fail a pending invocation if it's still waiting
    /// Used for timeouts and other error conditions
    pub fn fail_if_waiting(&self, req_id: &str, kind: &str, body: Vec<u8>) -> bool {
        self.complete(req_id, InvocationResult::err(kind, body))
    }
}