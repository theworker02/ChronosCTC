use thiserror::Error;

pub type BridgeResult<T> = Result<T, BridgeError>;

#[derive(Debug, Error)]
pub enum BridgeError {
    #[error("no compute device available for block class {0:?}")]
    NoDevice(crate::classify::BlockClass),

    #[error("device `{0}` is offline or saturated")]
    DeviceUnavailable(String),

    #[error("offload dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("kernel solve failed on device `{device}`: {source}")]
    DeviceSolve {
        device: String,
        #[source]
        source: ctc_kernel::KernelError,
    },

    #[error("bridge configuration error: {0}")]
    Config(String),

    #[error("work block `{0}` is empty")]
    EmptyBlock(String),
}
