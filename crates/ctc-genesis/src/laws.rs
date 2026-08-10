use crate::config::GenesisConfig;
use crate::error::{GenesisError, GenesisResult};
use serde::{Deserialize, Serialize};

/// Mutable physical / chronal constants of the execution universe.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PhysicalLaws {
    /// Deutsch residual tolerance \(\varepsilon\).
    pub deutsch_tolerance: f64,
    /// Chronal signalling speed multiplier (1.0 = nominal).
    pub signal_speed: f64,
    /// Holographic boundary compression ratio.
    pub boundary_ratio: f64,
    /// Manifold proper-time resolution (epochs per unit τ).
    pub manifold_resolution: f64,
    /// Anderson mixing β for the chronal kernel.
    pub anderson_beta: f64,
}

impl Default for PhysicalLaws {
    fn default() -> Self {
        Self {
            deutsch_tolerance: 1e-10,
            signal_speed: 1.0,
            boundary_ratio: 0.35,
            manifold_resolution: 1.0,
            anderson_beta: 1.0,
        }
    }
}

impl PhysicalLaws {
    pub fn as_vector(&self) -> Vec<f64> {
        vec![
            self.deutsch_tolerance.ln(), // log-space for scale
            self.signal_speed,
            self.boundary_ratio,
            self.manifold_resolution,
            self.anderson_beta,
        ]
    }

    pub fn from_vector(v: &[f64]) -> Self {
        Self {
            deutsch_tolerance: v.first().copied().unwrap_or(-23.0).exp(),
            signal_speed: v.get(1).copied().unwrap_or(1.0),
            boundary_ratio: v.get(2).copied().unwrap_or(0.35),
            manifold_resolution: v.get(3).copied().unwrap_or(1.0),
            anderson_beta: v.get(4).copied().unwrap_or(1.0),
        }
    }

    pub fn clamp_to(&mut self, cfg: &GenesisConfig) -> GenesisResult<()> {
        self.deutsch_tolerance = self
            .deutsch_tolerance
            .clamp(cfg.deutsch_tol_min, cfg.deutsch_tol_max);
        self.signal_speed = self
            .signal_speed
            .clamp(cfg.signal_speed_min, cfg.signal_speed_max);
        self.boundary_ratio = self
            .boundary_ratio
            .clamp(cfg.boundary_ratio_min, cfg.boundary_ratio_max);
        self.manifold_resolution = self.manifold_resolution.clamp(0.25, 8.0);
        self.anderson_beta = self.anderson_beta.clamp(0.1, 1.0);
        if self.deutsch_tolerance < cfg.deutsch_tol_min {
            return Err(GenesisError::LawOutOfRange("deutsch_tolerance".into()));
        }
        Ok(())
    }

    pub fn l2_distance(&self, other: &Self) -> f64 {
        let a = self.as_vector();
        let b = other.as_vector();
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| {
                let d = x - y;
                d * d
            })
            .sum::<f64>()
            .sqrt()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LawDelta {
    pub before: PhysicalLaws,
    pub after: PhysicalLaws,
    pub distance: f64,
}
