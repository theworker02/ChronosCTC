use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshConfig {
    /// Simulated one-way hop latency in microseconds (cost model).
    pub hop_latency_us: u64,
    /// Maximum envelopes buffered per entanglement channel.
    pub channel_capacity: usize,
    /// Require destination oracle injection point to be in Awaiting state.
    pub require_awaiting: bool,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            hop_latency_us: 10,
            channel_capacity: 256,
            require_awaiting: true,
        }
    }
}
