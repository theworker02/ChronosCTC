use thiserror::Error;

pub type ContinuumResult<T> = Result<T, ContinuumError>;

#[derive(Debug, Error)]
pub enum ContinuumError {
    #[error("region `{0}` is already admitted")]
    DuplicateRegion(String),

    #[error("region `{0}` is not admitted")]
    UnknownRegion(String),

    #[error("wormhole link failed: {0}")]
    Wormhole(#[from] ctc_wormhole::WormholeError),

    #[error("federation has no admitted regions")]
    EmptyFederation,

    #[error("tick budget exhausted at {0}")]
    TickBudget(u64),
}
