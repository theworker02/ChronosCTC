use thiserror::Error;

pub type CosmosResult<T> = Result<T, CosmosError>;

#[derive(Debug, Error)]
pub enum CosmosError {
    #[error("genesis bootstrap failed: {0}")]
    Genesis(String),

    #[error("holographic projection failed: {0}")]
    Holo(String),

    #[error("thermodynamic imbalance: {0}")]
    Entropy(String),

    #[error("kernel solve failed: {0}")]
    Kernel(String),

    #[error("horizon persistence failed: {0}")]
    Horizon(String),

    #[error("ledger error: {0}")]
    Ledger(String),

    #[error("collapse error: {0}")]
    Collapse(String),

    #[error("cosmos not sealed — call seal_laws before sustainment ticks")]
    NotSealed,

    #[error("equilibrium gate failed: residual {0:.6e}")]
    Equilibrium(f64),
}
