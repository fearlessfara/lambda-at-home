use lambda_control::{pending::Pending, queues::Queues};

#[derive(Clone)]
pub struct RtState {
    pub queues: Queues,
    pub pending: Pending,
}
