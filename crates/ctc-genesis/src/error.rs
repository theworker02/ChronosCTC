use thiserror::Error;

pub type GenesisResult<T> = Result<T, GenesisError>;

#[derive(Debug, Error)]
pub enum GenesisError {
    #[error("physical law parameter `{0}` out of admissible range")]
    LawOutOfRange(String),

    #[error("meta-compilation failed to converge after {0} epochs")]
    MetaDivergence(usize),

    #[error("bootstrap blocked: holographic/thermo precondition failed: {0}")]
    Precondition(String),

    #[error("holo error: {0}")]
    Holo(String),

    #[error("entropy error: {0}")]
    Entropy(String),

    #[error("kernel error during law evaluation: {0}")]
    Kernel(String),
}
