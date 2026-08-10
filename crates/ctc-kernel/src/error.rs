use thiserror::Error;

pub type KernelResult<T> = Result<T, KernelError>;

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("state dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Anderson history length m must be >= 1, got {0}")]
    InvalidAndersonDepth(usize),

    #[error("nonlinear system is empty — no chronal unknowns to solve")]
    EmptySystem,

    #[error("fixed-point iteration diverged: residual {residual:.6e} after {iterations} steps")]
    Diverged { residual: f64, iterations: usize },

    #[error("singular Anderson least-squares system (rank deficiency in residual history)")]
    SingularAndersonSystem,

    #[error("multi-start search exhausted without locating any fixed point")]
    NoFixedPointFound,
}
