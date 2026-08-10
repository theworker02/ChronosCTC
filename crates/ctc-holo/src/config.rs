use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HoloConfig {
    /// Target boundary dimension as a fraction of bulk dimension (0..1].
    pub boundary_ratio: f64,
    /// Minimum boundary cells even for tiny bulks.
    pub min_boundary_dim: usize,
    /// Maximum boundary cells (cap for large multiverses).
    pub max_boundary_dim: usize,
    /// Ryu–Takayanagi effective Newton constant \(G_N\) (encoding scale).
    pub newton_g: f64,
    /// Reconstruction tolerance when lifting boundary → bulk.
    pub reconstruct_tolerance: f64,
}

impl Default for HoloConfig {
    fn default() -> Self {
        Self {
            boundary_ratio: 0.35,
            min_boundary_dim: 2,
            max_boundary_dim: 256,
            newton_g: 1.0,
            reconstruct_tolerance: 1e-6,
        }
    }
}

impl HoloConfig {
    pub fn boundary_dim_for(&self, bulk_dim: usize) -> usize {
        if bulk_dim == 0 {
            return 0;
        }
        let target = ((bulk_dim as f64) * self.boundary_ratio).ceil() as usize;
        target
            .max(self.min_boundary_dim)
            .min(self.max_boundary_dim)
            .min(bulk_dim)
            .max(1)
    }
}
