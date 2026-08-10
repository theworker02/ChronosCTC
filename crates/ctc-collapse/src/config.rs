use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollapseConfig {
    /// Minimum score margin between winner and runner-up to accept collapse.
    pub consensus_margin: f64,
    /// Weight on probability mass in the utility score.
    pub w_probability: f64,
    /// Weight on inverse residual (consistency).
    pub w_consistency: f64,
    /// Weight on inverse entropy (stability).
    pub w_entropy: f64,
    /// Weight on agent health (fraction of clean probes).
    pub w_agent_health: f64,
    /// When true, prune non-winning live branches after synthesis.
    pub purge_losers: bool,
}

impl Default for CollapseConfig {
    fn default() -> Self {
        Self {
            consensus_margin: 0.02,
            w_probability: 0.45,
            w_consistency: 0.30,
            w_entropy: 0.15,
            w_agent_health: 0.10,
            purge_losers: true,
        }
    }
}
