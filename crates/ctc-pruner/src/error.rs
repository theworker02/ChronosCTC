use thiserror::Error;

pub type PrunerResult<T> = Result<T, PrunerError>;

#[derive(Debug, Error)]
pub enum PrunerError {
    #[error("branch {0:?} is not registered with the branch manager")]
    UnknownBranch(u64),

    #[error("no stable alternative branch available for collapse")]
    NoStableAlternative,

    #[error("worldline fabric error: {0}")]
    Dag(String),

    #[error("cannot fork branch: parent residual is already paradoxical")]
    ForkFromParadox,
}
