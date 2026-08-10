use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GcConfig {
    pub amplitude_floor: f64,
    pub toxic_residual: f64,
    pub seal_horizon: i64,
    pub max_live_branches: usize,
    pub heap_pressure_trigger: f64,
    /// Lagrange multiplier \(\lambda\) on residual in the entropy score.
    pub residual_entropy_weight: f64,
    pub max_checkpoints: usize,
}

impl Default for GcConfig {
    fn default() -> Self {
        Self {
            amplitude_floor: 1e-6,
            toxic_residual: 10.0,
            seal_horizon: 4,
            max_live_branches: 32,
            heap_pressure_trigger: 0.75,
            residual_entropy_weight: 0.35,
            max_checkpoints: 256,
        }
    }
}
