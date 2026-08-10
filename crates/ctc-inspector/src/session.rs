use crate::config::InspectorConfig;
use crate::error::{InspectorError, InspectorResult};
use crate::hotspot::{DivergenceHotspot, HotspotScanner};
use crate::manifold::{ManifoldSlice, ManifoldView};
use crate::telemetry::{TelemetryFrame, TelemetryRing};
use ctc_dag::WorldlineDag;
use ctc_kernel::IterationObserver;
use serde::{Deserialize, Serialize};

/// Proper-time scrub cursor for the spatial debugger.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct TauCursor {
    pub tau: i64,
    pub min: i64,
    pub max: i64,
}

impl TauCursor {
    pub fn scrub(&mut self, delta: i64) {
        self.tau = (self.tau + delta).clamp(self.min, self.max);
    }

    pub fn seek(&mut self, tau: i64) {
        self.tau = tau.clamp(self.min, self.max);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub cursor: TauCursor,
    pub slice: ManifoldSlice,
    pub latest_residual: Option<f64>,
    pub hotspots: Vec<DivergenceHotspot>,
    pub contraction_ratio: Option<f64>,
    pub ascii_manifold: String,
}

/// Interactive chronal debug session — attach to a worldline fabric and observe.
pub struct DebugSession {
    pub config: InspectorConfig,
    pub telemetry: TelemetryRing,
    pub hotspots: HotspotScanner,
    pub cursor: Option<TauCursor>,
    pub labels: Vec<String>,
}

impl DebugSession {
    pub fn new(config: InspectorConfig) -> Self {
        let telemetry = TelemetryRing::new(config.telemetry_capacity);
        let hotspots = HotspotScanner::new(&config);
        Self {
            config,
            telemetry,
            hotspots,
            cursor: None,
            labels: Vec::new(),
        }
    }

    pub fn set_labels(&mut self, labels: Vec<String>) {
        self.labels = labels;
    }

    /// Attach / refresh τ cursor bounds from the fabric.
    pub fn attach(&mut self, dag: &WorldlineDag) -> InspectorResult<()> {
        let view = ManifoldView::new(dag);
        let (min, max) = view.tau_range()?;
        let tau = self.cursor.map(|c| c.tau.clamp(min, max)).unwrap_or(min);
        self.cursor = Some(TauCursor { tau, min, max });
        Ok(())
    }

    pub fn scrub(&mut self, delta: i64) -> InspectorResult<()> {
        let c = self.cursor.as_mut().ok_or(InspectorError::NoFabric)?;
        c.scrub(delta);
        Ok(())
    }

    pub fn seek(&mut self, tau: i64) -> InspectorResult<()> {
        let c = self.cursor.as_mut().ok_or(InspectorError::NoFabric)?;
        c.seek(tau);
        Ok(())
    }

    pub fn snapshot(&self, dag: &WorldlineDag) -> InspectorResult<SessionSnapshot> {
        let cursor = self.cursor.ok_or(InspectorError::NoFabric)?;
        let view = ManifoldView::new(dag);
        let slice = view.slice(cursor.tau)?;
        let ascii_manifold = view.render_ascii(cursor.tau, self.config.tau_window)?;
        let latest = self.telemetry.latest();
        let hotspots = latest
            .as_ref()
            .map(|f| self.hotspots.scan(f, &self.labels))
            .unwrap_or_default();
        Ok(SessionSnapshot {
            cursor,
            slice,
            latest_residual: latest.as_ref().map(|f| f.residual_norm),
            hotspots,
            contraction_ratio: self.telemetry.contraction_ratio(4),
            ascii_manifold,
        })
    }

    pub fn latest_frame(&self) -> InspectorResult<TelemetryFrame> {
        self.telemetry
            .latest()
            .ok_or(InspectorError::EmptyTelemetry)
    }

    /// JSON export for external UI frontends.
    pub fn export_json(&self, dag: &WorldlineDag) -> InspectorResult<String> {
        let snap = self.snapshot(dag)?;
        serde_json::to_string_pretty(&snap).map_err(|e| InspectorError::Serde(e.to_string()))
    }

    /// Render a residual sparkline for the terminal.
    pub fn render_residual_sparkline(&self, width: usize) -> String {
        let trend = self.telemetry.trend();
        if trend.is_empty() {
            return "(no telemetry)".into();
        }
        let width = width.max(8);
        let step = (trend.len() / width).max(1);
        let vals: Vec<f64> = trend.iter().step_by(step).map(|(_, v)| *v).collect();
        let max_v = vals.iter().cloned().fold(0.0_f64, f64::max).max(1e-15);
        const LEVELS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let mut s = String::from("residual │");
        for v in vals.iter().take(width) {
            let idx = ((v / max_v) * (LEVELS.len() as f64 - 1.0)).round() as usize;
            s.push(LEVELS[idx.min(LEVELS.len() - 1)]);
        }
        if let Some((_, last)) = trend.last() {
            s.push_str(&format!("│ {:.3e}", last));
        }
        s
    }
}

impl IterationObserver for DebugSession {
    fn on_iteration(&mut self, telem: ctc_kernel::IterationTelemetry) {
        self.telemetry.on_iteration(telem);
    }
    fn on_restart(&mut self, restart: usize) {
        self.telemetry.on_restart(restart);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_dag::{NodeState, SpacetimeAddr};
    use ctc_kernel::IterationTelemetry;

    #[test]
    fn scrub_and_snapshot_manifold() {
        let mut dag = WorldlineDag::new();
        dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.1));
        dag.allocate(SpacetimeAddr::new(0, 1), NodeState::scalar(0.2));
        dag.allocate(SpacetimeAddr::new(0, 2), NodeState::scalar(0.3));

        let mut session = DebugSession::new(InspectorConfig::default());
        session.attach(&dag).unwrap();
        session.scrub(1).unwrap();
        let snap = session.snapshot(&dag).unwrap();
        assert_eq!(snap.cursor.tau, 1);
        assert!(snap.ascii_manifold.contains("τ1"));
    }

    #[test]
    fn telemetry_observer_records_hotspots() {
        let mut session = DebugSession::new(InspectorConfig {
            hotspot_sigma: 1.5,
            ..InspectorConfig::default()
        });
        session.set_labels(vec!["x".into(), "y".into()]);
        session.on_iteration(IterationTelemetry {
            iteration: 0,
            residual_norm: 3.0,
            max_abs_component: 3.0,
            state: vec![0.0, 0.0],
            residual: vec![0.1, 3.0],
        });
        let frame = session.latest_frame().unwrap();
        let hs = session.hotspots.scan(&frame, &session.labels);
        assert!(hs.iter().any(|h| h.label.as_deref() == Some("y")));
    }
}
