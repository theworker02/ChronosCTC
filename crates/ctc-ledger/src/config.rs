use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerConfig {
    /// Maximum live parallel universes before forced consensus pressure.
    pub max_live_universes: usize,
    /// Minimum probability weight to keep a child branch after bifurcation.
    pub min_branch_weight: f64,
    /// When true, append-only events are immutable even after collapse.
    pub retain_collapsed_history: bool,
}

impl Default for LedgerConfig {
    fn default() -> Self {
        Self {
            max_live_universes: 1024,
            min_branch_weight: 1e-9,
            retain_collapsed_history: true,
        }
    }
}
