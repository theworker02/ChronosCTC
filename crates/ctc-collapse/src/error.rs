use thiserror::Error;

pub type CollapseResult<T> = Result<T, CollapseError>;

#[derive(Debug, Error)]
pub enum CollapseError {
    #[error("no live universes available for consensus")]
    EmptyMultiverse,

    #[error("proof-of-consistency failed for universe `{0}`: {1}")]
    ProofFailed(u64, String),

    #[error("ledger error during collapse: {0}")]
    Ledger(String),

    #[error("cannot collapse: consensus score margin {margin:.4} below threshold {threshold:.4}")]
    AmbiguousConsensus { margin: f64, threshold: f64 },

    #[error("gc error during post-collapse purge: {0}")]
    Gc(String),
}
