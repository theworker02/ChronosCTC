use thiserror::Error;

pub type HoloResult<T> = Result<T, HoloError>;

#[derive(Debug, Error)]
pub enum HoloError {
    #[error("bulk state is empty — nothing to project onto the boundary")]
    EmptyBulk,

    #[error("boundary dimension {boundary} cannot encode bulk dimension {bulk}")]
    DimensionCollapse { bulk: usize, boundary: usize },

    #[error("entanglement matrix is singular / non-PSD")]
    SingularEntanglement,

    #[error("reconstruction residual {residual:.6e} exceeds tolerance {tolerance:.6e}")]
    ReconstructionFailed { residual: f64, tolerance: f64 },

    #[error("ledger error during holographic ingest: {0}")]
    Ledger(String),

    #[error("worldline fabric error: {0}")]
    Dag(String),
}
