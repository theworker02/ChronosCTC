use thiserror::Error;

pub type GcResult<T> = Result<T, GcError>;

#[derive(Debug, Error)]
pub enum GcError {
    #[error("cannot collect: retrocausal IPC pin held on node {0:?}")]
    PinnedNode(u64),

    #[error("checkpoint store overflow at capacity {0}")]
    CheckpointOverflow(usize),

    #[error("worldline fabric error during GC: {0}")]
    Dag(String),

    #[error("branch {0} is active and cannot be entropy-culled")]
    ActiveBranchProtected(u64),
}
