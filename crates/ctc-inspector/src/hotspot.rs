use crate::config::InspectorConfig;
use crate::telemetry::TelemetryFrame;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DivergenceHotspot {
    pub component: usize,
    pub value: f64,
    pub mean: f64,
    pub sigma_ratio: f64,
    pub label: Option<String>,
}

pub struct HotspotScanner {
    pub sigma: f64,
}

impl HotspotScanner {
    pub fn new(cfg: &InspectorConfig) -> Self {
        Self {
            sigma: cfg.hotspot_sigma,
        }
    }

    /// Flag residual components whose magnitude exceeds \(\sigma \cdot \mathrm{mean}(|r_i|)\).
    pub fn scan(&self, frame: &TelemetryFrame, labels: &[String]) -> Vec<DivergenceHotspot> {
        if frame.residual.is_empty() {
            return Vec::new();
        }
        let mean = frame.residual.iter().map(|v| v.abs()).sum::<f64>()
            / frame.residual.len() as f64;
        let threshold = self.sigma * mean.max(1e-15);

        frame
            .residual
            .iter()
            .enumerate()
            .filter(|(_, v)| v.abs() >= threshold && v.abs() > 1e-12)
            .map(|(i, v)| DivergenceHotspot {
                component: i,
                value: *v,
                mean,
                sigma_ratio: v.abs() / mean.max(1e-15),
                label: labels.get(i).cloned(),
            })
            .collect()
    }
}
