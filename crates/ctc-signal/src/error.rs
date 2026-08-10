use thiserror::Error;

pub type SignalResult<T> = Result<T, SignalError>;

#[derive(Debug, Error)]
pub enum SignalError {
    #[error("temporal packet footprint mismatch: expected {expected} cells, got {got}")]
    FootprintMismatch { expected: usize, got: usize },

    #[error("address layout mismatch at slot {slot}: expected {expected}, got {got}")]
    AddressMismatch {
        slot: usize,
        expected: String,
        got: String,
    },

    #[error("worldline binding hash mismatch: packet={packet:#x} live={live:#x}")]
    BindingMismatch { packet: u64, live: u64 },

    #[error("Deutsch residual {residual:.6e} exceeds tolerance {tolerance:.6e} after injection")]
    DeutschViolation { residual: f64, tolerance: f64 },

    #[error("cannot teleport: source epoch τ={from_tau} is not strictly after target τ={to_tau}")]
    NonRetrocausal { from_tau: i64, to_tau: i64 },

    #[error("injection target {0} is not allocated in the worldline fabric")]
    UnmappedTarget(String),

    #[error("packet rejected: sealed epoch at {0}")]
    SealedTarget(String),

    #[error("empty teleportation payload")]
    EmptyPayload,

    #[error("worldline fabric error: {0}")]
    Dag(String),

    #[error("kernel residual evaluation failed: {0}")]
    Kernel(String),
}
