use thiserror::Error;

pub type WormholeResult<T> = Result<T, WormholeError>;

#[derive(Debug, Error)]
pub enum WormholeError {
    #[error("wormhole portal `{0}` is closed")]
    PortalClosed(u64),

    #[error("wormhole portal `{portal_id}` queue is full (capacity {capacity})")]
    QueueFull { portal_id: u64, capacity: usize },

    #[error("no packet available on portal `{portal_id}` for endpoint `{endpoint}`")]
    NoPacket { portal_id: u64, endpoint: String },

    #[error("endpoint mismatch: portal `{portal_id}` expects `{expected}`, got `{actual}`")]
    EndpointMismatch {
        portal_id: u64,
        expected: String,
        actual: String,
    },

    #[error("unknown portal `{0}`")]
    UnknownPortal(u64),
}
