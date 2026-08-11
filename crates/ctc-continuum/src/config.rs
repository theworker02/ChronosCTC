use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinuumConfig {
    pub max_regions: usize,
    pub portal_capacity: usize,
    pub federation_ticks: u64,
}

impl Default for ContinuumConfig {
    fn default() -> Self {
        Self {
            max_regions: 8,
            portal_capacity: 16,
            federation_ticks: 4,
        }
    }
}
