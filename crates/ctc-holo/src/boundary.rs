use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryTopology {
    /// Flat 2D lattice torus \(S^1 \times S^1\).
    Torus2d,
    /// Disk with radial AdS-like warping.
    AdSDisk,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundaryCell {
    pub u: f64,
    pub v: f64,
    pub amplitude: f64,
    pub phase: f64,
}

/// Lower-dimensional holographic screen holding the encoded bulk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundarySurface {
    pub topology: BoundaryTopology,
    pub dim: usize,
    pub cells: Vec<BoundaryCell>,
    /// Bulk dimension prior to projection.
    pub bulk_dim: usize,
    /// Fingerprint of the source bulk for integrity checks.
    pub bulk_fingerprint: u64,
}

impl BoundarySurface {
    pub fn entropy_shannon(&self) -> f64 {
        let norms: Vec<f64> = self
            .cells
            .iter()
            .map(|c| c.amplitude.abs())
            .collect();
        let sum: f64 = norms.iter().sum();
        if sum <= 0.0 {
            return 0.0;
        }
        norms
            .iter()
            .filter(|p| **p > 0.0)
            .map(|p| {
                let q = *p / sum;
                -q * q.ln()
            })
            .sum()
    }

    pub fn area(&self) -> f64 {
        // Effective holographic area ∝ number of active cells × mean amplitude.
        let active = self.cells.iter().filter(|c| c.amplitude.abs() > 1e-15).count() as f64;
        let mean = if self.cells.is_empty() {
            0.0
        } else {
            self.cells.iter().map(|c| c.amplitude.abs()).sum::<f64>() / self.cells.len() as f64
        };
        active * (1.0 + mean)
    }
}
