//! Anderson acceleration for fixed-point iteration.
//!
//! Given iterates \(x_k\) and residuals \(r_k = F(x_k) - x_k\), Anderson(\(m\))
//! maintains the last \(m\) residual differences and solves the unconstrained
//! least-squares problem
//!
//! \[
//! \min_{\alpha \in \mathbb{R}^{m}}
//! \left\| r_k - \sum_{i=1}^{m} \alpha_i (r_{k-i+1} - r_{k-i}) \right\|_2
//! \]
//!
//! Then forms the accelerated step
//!
//! \[
//! x_{k+1}
//! =
//! x_k + \beta r_k
//! - \sum_{i=1}^{m} \alpha_i \big( (x_{k-i+1} - x_{k-i}) + \beta (r_{k-i+1} - r_{k-i}) \big)
//! \]
//!
//! with mixing parameter \(\beta \in (0, 1]\).

use crate::error::{KernelError, KernelResult};
use nalgebra::{DMatrix, DVector};

pub struct AndersonAccelerator {
    m: usize,
    beta: f64,
    xs: Vec<DVector<f64>>,
    rs: Vec<DVector<f64>>,
}

impl AndersonAccelerator {
    pub fn new(m: usize, beta: f64) -> KernelResult<Self> {
        if m == 0 {
            return Err(KernelError::InvalidAndersonDepth(m));
        }
        Ok(Self {
            m,
            beta: beta.clamp(1e-6, 1.0),
            xs: Vec::with_capacity(m + 1),
            rs: Vec::with_capacity(m + 1),
        })
    }

    pub fn reset(&mut self) {
        self.xs.clear();
        self.rs.clear();
    }

    /// Push \((x, r)\) and return the accelerated next iterate.
    pub fn step(&mut self, x: DVector<f64>, r: DVector<f64>) -> KernelResult<DVector<f64>> {
        if !self.xs.is_empty() && self.xs[0].len() != x.len() {
            return Err(KernelError::DimensionMismatch {
                expected: self.xs[0].len(),
                got: x.len(),
            });
        }

        self.xs.push(x);
        self.rs.push(r);
        if self.xs.len() > self.m + 1 {
            self.xs.remove(0);
            self.rs.remove(0);
        }

        let k = self.xs.len();
        if k < 2 {
            // Picard step: x ← x + β r
            let x0 = &self.xs[0];
            let r0 = &self.rs[0];
            return Ok(x0 + r0.scale(self.beta));
        }

        let mk = (k - 1).min(self.m);
        let dim = self.xs[0].len();
        let rk = &self.rs[k - 1];

        // Build ΔR columns: r_{j+1} - r_j for the last mk pairs.
        let mut delta_r = DMatrix::<f64>::zeros(dim, mk);
        let mut delta_x = DMatrix::<f64>::zeros(dim, mk);
        for i in 0..mk {
            let idx = k - 1 - mk + i;
            let dr = &self.rs[idx + 1] - &self.rs[idx];
            let dx = &self.xs[idx + 1] - &self.xs[idx];
            delta_r.set_column(i, &dr);
            delta_x.set_column(i, &dx);
        }

        // Solve min || ΔR α - r_k || via normal equations (ΔRᵀ ΔR) α = ΔRᵀ r_k.
        // Constant residuals (e.g. F(x)=x+c) make ΔR rank-deficient — fall back
        // to a Picard step so the outer residual monitor can detect divergence.
        let gram = delta_r.transpose() * &delta_r;
        let rhs = delta_r.transpose() * rk;
        let xk = &self.xs[k - 1];

        match gram.lu().solve(&rhs) {
            Some(alpha) => {
                let correction = (&delta_x + delta_r.scale(self.beta)) * alpha;
                Ok(xk + rk.scale(self.beta) - correction)
            }
            None => Ok(xk + rk.scale(self.beta)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anderson_picard_on_first_step() {
        let mut acc = AndersonAccelerator::new(3, 1.0).unwrap();
        let x = DVector::from_vec(vec![1.0, 2.0]);
        let r = DVector::from_vec(vec![-0.5, 0.0]);
        let next = acc.step(x, r).unwrap();
        assert!((next[0] - 0.5).abs() < 1e-12);
        assert!((next[1] - 2.0).abs() < 1e-12);
    }
}
