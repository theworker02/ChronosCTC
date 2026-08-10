//! Chronal fixed-point solver.
//!
//! ## Multi-start basin search
//!
//! Because CTC maps may admit multiple fixed points, the kernel launches
//! `num_restarts` independent Anderson trajectories from stratified initial
//! conditions. Attractors are clustered by L2 distance `< cluster_eps`; each
//! cluster receives probability weight proportional to the number of
//! trajectories that landed in it (empirical basin measure).

use crate::anderson::AndersonAccelerator;
use crate::error::{KernelError, KernelResult};
use crate::evolution::NonlinearSystem;
use crate::residual::ResidualMonitor;
use nalgebra::DVector;
use serde::{Deserialize, Serialize};

/// Per-iteration telemetry emitted by observed solves.
#[derive(Clone, Debug)]
pub struct IterationTelemetry {
    pub iteration: usize,
    pub residual_norm: f64,
    pub max_abs_component: f64,
    pub state: Vec<f64>,
    pub residual: Vec<f64>,
}

/// Hook for live residual / state streaming (`ctc-inspector`, `ctc-bridge`).
pub trait IterationObserver {
    fn on_iteration(&mut self, telem: IterationTelemetry);
    fn on_restart(&mut self, _restart: usize) {}
}

impl IterationObserver for () {
    fn on_iteration(&mut self, _telem: IterationTelemetry) {}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverConfig {
    pub max_iterations: usize,
    pub tolerance: f64,
    pub anderson_m: usize,
    pub anderson_beta: f64,
    pub num_restarts: usize,
    pub cluster_eps: f64,
    /// Domain box \([lo, hi]^n\) for multi-start sampling.
    pub domain_lo: f64,
    pub domain_hi: f64,
}

impl Default for SolverConfig {
    fn default() -> Self {
        Self {
            max_iterations: 256,
            tolerance: 1e-10,
            anderson_m: 5,
            anderson_beta: 1.0,
            num_restarts: 8,
            cluster_eps: 1e-6,
            domain_lo: 0.0,
            domain_hi: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolverStats {
    pub iterations: usize,
    pub final_residual: f64,
    pub restarts_used: usize,
    pub fixed_points_found: usize,
}

/// Classification of the Deutsch fixed-point landscape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConvergenceClass {
    /// Unique \(x^\star\) with \(U(\rho)=\rho\).
    Unique,
    /// Several fixed points; weights sum to 1.
    MultiWeighted,
    /// Empty fixed-point set — logical paradox.
    Paradox,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixedPointSolution {
    pub class: ConvergenceClass,
    /// Converged state vectors (one per attractor cluster).
    pub states: Vec<Vec<f64>>,
    /// Probability weights over `states` (Deutsch mixture).
    pub weights: Vec<f64>,
    pub stats: SolverStats,
}

pub struct ChronalKernel {
    pub config: SolverConfig,
}

impl Default for ChronalKernel {
    fn default() -> Self {
        Self {
            config: SolverConfig::default(),
        }
    }
}

impl ChronalKernel {
    pub fn new(config: SolverConfig) -> Self {
        Self { config }
    }

    /// Solve \(x = F(x)\) under the configured Anderson / multi-start policy.
    pub fn solve(&self, system: &NonlinearSystem) -> KernelResult<FixedPointSolution> {
        let dim = system.dimension();
        if dim == 0 {
            return Err(KernelError::EmptySystem);
        }

        let mut attractors: Vec<(DVector<f64>, usize)> = Vec::new();
        let mut total_iters = 0usize;
        let mut best_residual = f64::INFINITY;

        for restart in 0..self.config.num_restarts {
            let x0 = self.initial_guess(dim, restart);
            match self.solve_once(system, x0) {
                Ok((x_star, iters, resid)) => {
                    total_iters += iters;
                    best_residual = best_residual.min(resid);
                    self.insert_attractor(&mut attractors, x_star);
                }
                Err(KernelError::Diverged { residual, iterations }) => {
                    total_iters += iterations;
                    best_residual = best_residual.min(residual);
                }
                Err(e) => return Err(e),
            }
        }

        let stats = SolverStats {
            iterations: total_iters,
            final_residual: best_residual,
            restarts_used: self.config.num_restarts,
            fixed_points_found: attractors.len(),
        };

        if attractors.is_empty() {
            return Ok(FixedPointSolution {
                class: ConvergenceClass::Paradox,
                states: vec![],
                weights: vec![],
                stats,
            });
        }

        let total_hits: usize = attractors.iter().map(|(_, h)| *h).sum();
        let weights: Vec<f64> = attractors
            .iter()
            .map(|(_, h)| *h as f64 / total_hits as f64)
            .collect();
        let states: Vec<Vec<f64>> = attractors
            .into_iter()
            .map(|(v, _)| v.iter().copied().collect())
            .collect();

        let class = if states.len() == 1 {
            ConvergenceClass::Unique
        } else {
            ConvergenceClass::MultiWeighted
        };

        Ok(FixedPointSolution {
            class,
            states,
            weights,
            stats,
        })
    }

    /// Single-trajectory Anderson solve from `x0`.
    pub fn solve_once(
        &self,
        system: &NonlinearSystem,
        x: DVector<f64>,
    ) -> KernelResult<(DVector<f64>, usize, f64)> {
        self.solve_once_observed(system, x, &mut ())
    }

    /// Like [`solve`](Self::solve) but streams per-iteration telemetry to `obs`.
    ///
    /// Used by `ctc-inspector` for live residual manifolds and by `ctc-bridge`
    /// for mid-flight device migration decisions.
    pub fn solve_observed<O: IterationObserver>(
        &self,
        system: &NonlinearSystem,
        obs: &mut O,
    ) -> KernelResult<FixedPointSolution> {
        let dim = system.dimension();
        if dim == 0 {
            return Err(KernelError::EmptySystem);
        }

        let mut attractors: Vec<(DVector<f64>, usize)> = Vec::new();
        let mut total_iters = 0usize;
        let mut best_residual = f64::INFINITY;

        for restart in 0..self.config.num_restarts {
            obs.on_restart(restart);
            let x0 = self.initial_guess(dim, restart);
            match self.solve_once_observed(system, x0, obs) {
                Ok((x_star, iters, resid)) => {
                    total_iters += iters;
                    best_residual = best_residual.min(resid);
                    self.insert_attractor(&mut attractors, x_star);
                }
                Err(KernelError::Diverged { residual, iterations }) => {
                    total_iters += iterations;
                    best_residual = best_residual.min(residual);
                }
                Err(e) => return Err(e),
            }
        }

        let stats = SolverStats {
            iterations: total_iters,
            final_residual: best_residual,
            restarts_used: self.config.num_restarts,
            fixed_points_found: attractors.len(),
        };

        if attractors.is_empty() {
            return Ok(FixedPointSolution {
                class: ConvergenceClass::Paradox,
                states: vec![],
                weights: vec![],
                stats,
            });
        }

        let total_hits: usize = attractors.iter().map(|(_, h)| *h).sum();
        let weights: Vec<f64> = attractors
            .iter()
            .map(|(_, h)| *h as f64 / total_hits as f64)
            .collect();
        let states: Vec<Vec<f64>> = attractors
            .into_iter()
            .map(|(v, _)| v.iter().copied().collect())
            .collect();

        let class = if states.len() == 1 {
            ConvergenceClass::Unique
        } else {
            ConvergenceClass::MultiWeighted
        };

        Ok(FixedPointSolution {
            class,
            states,
            weights,
            stats,
        })
    }

    /// Single-trajectory Anderson solve with per-iteration observation.
    pub fn solve_once_observed<O: IterationObserver>(
        &self,
        system: &NonlinearSystem,
        mut x: DVector<f64>,
        obs: &mut O,
    ) -> KernelResult<(DVector<f64>, usize, f64)> {
        let mut accel =
            AndersonAccelerator::new(self.config.anderson_m, self.config.anderson_beta)?;
        let mut monitor = ResidualMonitor::new(self.config.tolerance);

        for iter in 0..self.config.max_iterations {
            let r = system.residual(&x)?;
            let sample = monitor.push(iter, &r);
            let state_view: Vec<f64> = x.iter().copied().collect();
            let resid_view: Vec<f64> = r.iter().copied().collect();
            obs.on_iteration(IterationTelemetry {
                iteration: iter,
                residual_norm: sample.norm,
                max_abs_component: sample.max_abs_component,
                state: state_view,
                residual: resid_view,
            });
            if sample.norm <= self.config.tolerance {
                return Ok((x, iter + 1, sample.norm));
            }
            if monitor.diverging() {
                return Err(KernelError::Diverged {
                    residual: sample.norm,
                    iterations: iter + 1,
                });
            }
            x = accel.step(x, r)?;
        }

        let final_r = monitor.last_norm();
        if final_r <= self.config.tolerance {
            Ok((x, self.config.max_iterations, final_r))
        } else {
            Err(KernelError::Diverged {
                residual: final_r,
                iterations: self.config.max_iterations,
            })
        }
    }

    fn initial_guess(&self, dim: usize, restart: usize) -> DVector<f64> {
        let lo = self.config.domain_lo;
        let hi = self.config.domain_hi;
        if self.config.num_restarts == 1 {
            return DVector::from_element(dim, 0.5 * (lo + hi));
        }
        // Stratified lattice along the diagonal + small coordinate jitter.
        let t = restart as f64 / (self.config.num_restarts as f64 - 1.0).max(1.0);
        let base = lo + t * (hi - lo);
        let mut v = DVector::from_element(dim, base);
        for i in 0..dim {
            let jitter = ((restart * 17 + i * 31) % 1000) as f64 / 1000.0;
            v[i] = (lo + (base - lo + jitter * (hi - lo) * 0.05)).clamp(lo, hi);
        }
        v
    }

    fn insert_attractor(&self, attractors: &mut Vec<(DVector<f64>, usize)>, x: DVector<f64>) {
        for (existing, hits) in attractors.iter_mut() {
            if (&*existing - &x).norm() < self.config.cluster_eps {
                *hits += 1;
                // Running mean keeps the representative inside the basin.
                let n = *hits as f64;
                *existing = existing.scale((n - 1.0) / n) + x.scale(1.0 / n);
                return;
            }
        }
        attractors.push((x, 1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution::{AffineEvolution, FnEvolution, NonlinearSystem};
    use nalgebra::{DMatrix, DVector};
    use std::sync::Arc;

    #[test]
    fn unique_fixed_point_of_contraction() {
        // F(x) = 0.5 x + 1  =>  x* = 2
        let evo = Arc::new(FnEvolution::new(1, |x| {
            DVector::from_vec(vec![0.5 * x[0] + 1.0])
        }));
        let sys = NonlinearSystem::new("contract", evo, vec!["x".into()]).unwrap();
        let sol = ChronalKernel::default().solve(&sys).unwrap();
        assert_eq!(sol.class, ConvergenceClass::Unique);
        assert!((sol.states[0][0] - 2.0).abs() < 1e-8);
    }

    #[test]
    fn paradox_negation_has_no_real_bit_fixed_point_under_tight_domain() {
        // Classical bit negation on [0,1]: F(x)=1-x has fixed point x=0.5,
        // which IS consistent in the continuous embedding. The true logical
        // paradox for bits is enforced by projecting to {0,1}.
        // Here we use F(x) = x + 1 — no fixed point anywhere.
        let evo = Arc::new(AffineEvolution {
            a: DMatrix::identity(1, 1),
            b: DVector::from_vec(vec![1.0]),
        });
        let sys = NonlinearSystem::new("shift", evo, vec!["x".into()]).unwrap();
        let mut cfg = SolverConfig::default();
        cfg.max_iterations = 32;
        cfg.num_restarts = 4;
        cfg.tolerance = 1e-12;
        let sol = ChronalKernel::new(cfg).solve(&sys).unwrap();
        assert_eq!(sol.class, ConvergenceClass::Paradox);
    }

    #[test]
    fn multi_weighted_identity_basins() {
        // F(x) = x — every point is a fixed point. Multi-start with tight
        // clustering yields a Deutsch mixture over distinct landings.
        let evo = Arc::new(FnEvolution::new(1, |x| x.clone()));
        let sys = NonlinearSystem::new("identity", evo, vec!["x".into()]).unwrap();
        let mut cfg = SolverConfig::default();
        cfg.num_restarts = 4;
        cfg.cluster_eps = 1e-3;
        cfg.domain_lo = 0.0;
        cfg.domain_hi = 1.0;
        let sol = ChronalKernel::new(cfg).solve(&sys).unwrap();
        assert_eq!(sol.class, ConvergenceClass::MultiWeighted);
        assert!(sol.states.len() > 1);
        let wsum: f64 = sol.weights.iter().sum();
        assert!((wsum - 1.0).abs() < 1e-9);
    }
}
