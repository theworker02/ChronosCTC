use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Residual above which an agent flags a paradox risk.
    pub paradox_residual: f64,
    /// Weight below which a branch is considered sub-optimal.
    pub suboptimal_weight: f64,
    /// Magnitude scale for correction vectors injected into the past.
    pub correction_scale: f64,
    /// Maximum probes an agent executes per deployment.
    pub max_probes_per_tick: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            paradox_residual: 1.0,
            suboptimal_weight: 0.05,
            correction_scale: 0.01,
            max_probes_per_tick: 8,
        }
    }
}
