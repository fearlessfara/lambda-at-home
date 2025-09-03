use tokio::sync::mpsc;
use crate::queues::Queues;
use crate::work_item::WorkItem;
use crate::pending::Pending;
use tracing::{info, warn};

#[derive(Clone)]
pub struct Scheduler {
    queues: Queues,
    pending: Pending,
    tx: mpsc::Sender<WorkItem>,
}

impl Scheduler {
    pub fn new() -> (Self, mpsc::Receiver<WorkItem>) {
        let (tx, rx) = mpsc::channel(1024); // bounded global queue
        let queues = Queues::new();
        let pending = Pending::new();
        
        (Self { queues, pending, tx }, rx)
    }
    
    pub fn queues(&self) -> Queues {
        self.queues.clone()
    }
    
    pub fn pending(&self) -> Pending {
        self.pending.clone()
    }
    
    pub async fn enqueue(&self, wi: WorkItem) -> anyhow::Result<()> {
        self.tx.send(wi).await.map_err(|e| anyhow::anyhow!(e))
    }
}

/// Spawn once at app start - dispatcher task that fans out from global queue to per-function queues
pub async fn run_dispatcher(mut rx: mpsc::Receiver<WorkItem>, queues: Queues) {
    info!("Dispatcher task started");
    
    while let Some(wi) = rx.recv().await {
        if let Err(e) = queues.push(wi) {
            warn!(error=?e, "failed to push to per-fn queue");
        }
    }
    
    info!("Dispatcher task exiting");
}