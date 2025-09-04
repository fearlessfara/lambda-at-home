use std::sync::Arc;
use lambda_control::{pending::Pending, queues::Queues, ControlPlane};

#[derive(Clone)]
pub struct RtState {
    // Optional control plane. If present, handlers should use it for
    // shared queues/pending. When None (e.g., in unit tests), fall back
    // to local Queues/Pending below.
    pub control: Option<Arc<ControlPlane>>,
    pub queues: Queues,
    pub pending: Pending,
}
