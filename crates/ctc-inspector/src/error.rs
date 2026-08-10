use thiserror::Error;

pub type InspectorResult<T> = Result<T, InspectorError>;

#[derive(Debug, Error)]
pub enum InspectorError {
    #[error("τ={requested} is outside the allocated worldline range [{min}, {max}]")]
    TauOutOfRange { requested: i64, min: i64, max: i64 },

    #[error("debug session has no attached worldline fabric")]
    NoFabric,

    #[error("telemetry ring is empty — no residual samples captured yet")]
    EmptyTelemetry,

    #[error("serialization error: {0}")]
    Serde(String),
}
