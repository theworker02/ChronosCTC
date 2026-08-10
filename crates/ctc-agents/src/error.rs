use thiserror::Error;

pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("agent `{0}` is not registered in the fleet")]
    UnknownAgent(u64),

    #[error("ledger error during agent navigation: {0}")]
    Ledger(String),

    #[error("signal injection failed: {0}")]
    Signal(String),

    #[error("target universe `{0}` is not navigable")]
    UnnavigableUniverse(u64),

    #[error("agent `{0}` is already decommissioned")]
    Decommissioned(u64),
}
