use ctc_entropy::EnergyLedger;
use ctc_genesis::PhysicalLaws;
use ctc_holo::BoundarySurface;
use serde::{Deserialize, Serialize};

/// Frozen cosmos state that survives process death — past the event horizon.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CosmosCheckpoint {
    pub id: u64,
    pub tick: u64,
    pub laws: PhysicalLaws,
    pub energy: EnergyLedger,
    pub boundary: Option<BoundarySurface>,
    /// Flattened primary-manifold bulk at seal time.
    pub primary_bulk: Vec<f64>,
    pub kernel_residual: f64,
    pub zero_energy: bool,
    pub note: String,
}

impl CosmosCheckpoint {
    pub fn fingerprint(&self) -> u64 {
        use rustc_hash::FxHasher;
        use std::hash::{Hash, Hasher};
        let mut h = FxHasher::default();
        self.tick.hash(&mut h);
        self.primary_bulk.len().hash(&mut h);
        for v in &self.primary_bulk {
            v.to_bits().hash(&mut h);
        }
        self.laws.deutsch_tolerance.to_bits().hash(&mut h);
        self.laws.signal_speed.to_bits().hash(&mut h);
        h.finish()
    }
}
