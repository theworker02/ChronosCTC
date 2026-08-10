use thiserror::Error;

pub type HorizonResult<T> = Result<T, HorizonError>;

#[derive(Debug, Error)]
pub enum HorizonError {
    #[error("horizon checkpoint I/O: {0}")]
    Io(String),

    #[error("horizon checkpoint corrupt or incompatible: {0}")]
    Corrupt(String),

    #[error("horizon store capacity exceeded ({0})")]
    Capacity(usize),

    #[error("checkpoint id {0} not found beyond the event horizon")]
    Missing(u64),
}
