use thiserror::Error;

pub type OracleResult<T> = Result<T, OracleError>;

#[derive(Debug, Error)]
pub enum OracleError {
    #[error("injection point `{0}` is not registered")]
    UnknownInjectionPoint(String),

    #[error("superposition at `{0}` timed out waiting for retrocausal signal")]
    WaitTimeout(String),

    #[error("cannot collapse `{0}`: superposition is not awaiting a packet")]
    InvalidCollapse(String),

    #[error("signal transport error: {0}")]
    Signal(String),

    #[error("injection point `{0}` already collapsed")]
    AlreadyCollapsed(String),

    #[error("worldline fabric error: {0}")]
    Dag(String),
}
