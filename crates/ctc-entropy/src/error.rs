use thiserror::Error;

pub type EntropyResult<T> = Result<T, EntropyError>;

#[derive(Debug, Error)]
pub enum EntropyError {
    #[error("temperature must be positive, got {0}")]
    InvalidTemperature(f64),

    #[error("energy ledger underflow: available={available:.6e} required={required:.6e}")]
    EnergyUnderflow { available: f64, required: f64 },

    #[error("holo coupling error: {0}")]
    Holo(String),

    #[error("thermodynamic equilibrium not reachable: residual={0:.6e}")]
    EquilibriumUnreachable(f64),
}
