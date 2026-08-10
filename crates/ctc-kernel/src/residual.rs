use serde::{Deserialize, Serialize};

/// One sample of the residual trajectory \(\|r(x_k)\|_2\).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResidualSample {
    pub iteration: usize,
    pub norm: f64,
    pub max_abs_component: f64,
}

/// Rolling residual monitor used by both the kernel and Paradox Pruner.
///
/// Divergence heuristic: if the residual fails to decrease by factor
/// `stagnation_ratio` over `stagnation_window` iterations, or exceeds
/// `divergence_ceiling`, the monitor flags a paradox candidate.
#[derive(Clone, Debug)]
pub struct ResidualMonitor {
    pub history: Vec<ResidualSample>,
    pub tolerance: f64,
    pub divergence_ceiling: f64,
    pub stagnation_window: usize,
    pub stagnation_ratio: f64,
}

impl ResidualMonitor {
    pub fn new(tolerance: f64) -> Self {
        Self {
            history: Vec::new(),
            tolerance,
            divergence_ceiling: 1e6,
            stagnation_window: 16,
            stagnation_ratio: 0.99,
        }
    }

    pub fn push(&mut self, iteration: usize, residual: &nalgebra::DVector<f64>) -> ResidualSample {
        let norm = residual.norm();
        let max_abs_component = residual.amax();
        let sample = ResidualSample {
            iteration,
            norm,
            max_abs_component,
        };
        self.history.push(sample.clone());
        sample
    }

    pub fn converged(&self) -> bool {
        self.history
            .last()
            .map(|s| s.norm <= self.tolerance)
            .unwrap_or(false)
    }

    pub fn diverging(&self) -> bool {
        let Some(last) = self.history.last() else {
            return false;
        };
        if last.norm > self.divergence_ceiling {
            return true;
        }
        if self.history.len() < self.stagnation_window + 1 {
            return false;
        }
        let n = self.history.len();
        let older = self.history[n - 1 - self.stagnation_window].norm;
        let newer = last.norm;
        // No meaningful contraction and still far from tolerance.
        newer > self.tolerance * 10.0 && newer >= older * self.stagnation_ratio
    }

    pub fn last_norm(&self) -> f64 {
        self.history.last().map(|s| s.norm).unwrap_or(f64::INFINITY)
    }
}
