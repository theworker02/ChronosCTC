use ctc_kernel::{IterationObserver, IterationTelemetry};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryFrame {
    pub restart: usize,
    pub iteration: usize,
    pub residual_norm: f64,
    pub max_abs_component: f64,
    pub residual: Vec<f64>,
    pub state: Vec<f64>,
}

/// Lock-free-read ring buffer of residual samples for live UI streaming.
pub struct TelemetryRing {
    capacity: usize,
    frames: RwLock<VecDeque<TelemetryFrame>>,
    current_restart: RwLock<usize>,
}

impl TelemetryRing {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            frames: RwLock::new(VecDeque::with_capacity(capacity.min(1024))),
            current_restart: RwLock::new(0),
        }
    }

    pub fn len(&self) -> usize {
        self.frames.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.read().is_empty()
    }

    pub fn push(&self, frame: TelemetryFrame) {
        let mut q = self.frames.write();
        if q.len() >= self.capacity {
            q.pop_front();
        }
        q.push_back(frame);
    }

    pub fn latest(&self) -> Option<TelemetryFrame> {
        self.frames.read().back().cloned()
    }

    pub fn trend(&self) -> Vec<(usize, f64)> {
        self.frames
            .read()
            .iter()
            .map(|f| (f.iteration, f.residual_norm))
            .collect()
    }

    /// Residual contraction ratio over the last `window` samples.
    pub fn contraction_ratio(&self, window: usize) -> Option<f64> {
        let q = self.frames.read();
        if q.len() < window + 1 || window == 0 {
            return None;
        }
        let n = q.len();
        let older = q[n - 1 - window].residual_norm;
        let newer = q[n - 1].residual_norm;
        if older <= f64::EPSILON {
            return Some(0.0);
        }
        Some(newer / older)
    }

    pub fn clear(&self) {
        self.frames.write().clear();
    }
}

impl IterationObserver for TelemetryRing {
    fn on_iteration(&mut self, telem: IterationTelemetry) {
        let restart = *self.current_restart.read();
        self.push(TelemetryFrame {
            restart,
            iteration: telem.iteration,
            residual_norm: telem.residual_norm,
            max_abs_component: telem.max_abs_component,
            residual: telem.residual,
            state: telem.state,
        });
    }

    fn on_restart(&mut self, restart: usize) {
        *self.current_restart.write() = restart;
    }
}
