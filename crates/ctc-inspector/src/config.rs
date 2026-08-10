use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorConfig {
    pub telemetry_capacity: usize,
    pub hotspot_sigma: f64,
    pub tau_window: i64,
}

impl Default for InspectorConfig {
    fn default() -> Self {
        Self {
            telemetry_capacity: 4096,
            hotspot_sigma: 2.5,
            tau_window: 2,
        }
    }
}
