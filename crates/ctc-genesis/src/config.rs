use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Meta-compilation learning rate for law updates.
    pub law_learn_rate: f64,
    /// Maximum bootstrap epochs seeking \(\Lambda^\star = G(W(\Lambda))\).
    pub max_meta_epochs: usize,
    /// Convergence tolerance on law-vector L2 delta.
    pub law_tolerance: f64,
    /// Admissible Deutsch tolerance range.
    pub deutsch_tol_min: f64,
    pub deutsch_tol_max: f64,
    /// Admissible chronal signal speed multiplier range.
    pub signal_speed_min: f64,
    pub signal_speed_max: f64,
    /// Admissible holographic boundary ratio range.
    pub boundary_ratio_min: f64,
    pub boundary_ratio_max: f64,
}

impl Default for GenesisConfig {
    fn default() -> Self {
        Self {
            law_learn_rate: 0.35,
            max_meta_epochs: 12,
            law_tolerance: 1e-6,
            deutsch_tol_min: 1e-14,
            deutsch_tol_max: 1e-4,
            signal_speed_min: 0.25,
            signal_speed_max: 4.0,
            boundary_ratio_min: 0.15,
            boundary_ratio_max: 0.75,
        }
    }
}
