use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntropyConfig {
    /// Absolute temperature \(T\) in kelvin for Landauer bound.
    pub temperature_k: f64,
    /// Boltzmann constant override (default SI).
    pub boltzmann_j_per_k: f64,
    /// Energy harvested per unit residual reduction (simulation units).
    pub harvest_per_residual: f64,
    /// When residual below this, treat as zero-energy convergence state.
    pub zero_energy_residual: f64,
}

impl Default for EntropyConfig {
    fn default() -> Self {
        Self {
            temperature_k: 300.0,
            boltzmann_j_per_k: 1.380_649e-23,
            harvest_per_residual: 1e-21,
            zero_energy_residual: 1e-12,
        }
    }
}
