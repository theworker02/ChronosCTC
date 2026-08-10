use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CosmosConfig {
    /// Sustainment ticks after Λ* seal.
    pub sustain_ticks: usize,
    /// Re-run genesis meta-compile if residual drifts above this multiple of ε.
    pub drift_recompile_factor: f64,
    /// Persist a horizon checkpoint every N ticks (0 = only at seal / final).
    pub checkpoint_every: usize,
    /// Horizon ring capacity.
    pub horizon_capacity: usize,
    /// Enforce Landauer zero-energy gate at end of sustainment.
    pub require_zero_energy: bool,
}

impl Default for CosmosConfig {
    fn default() -> Self {
        Self {
            sustain_ticks: 4,
            drift_recompile_factor: 1e3,
            checkpoint_every: 2,
            horizon_capacity: 32,
            require_zero_energy: true,
        }
    }
}
