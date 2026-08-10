use crate::config::GenesisConfig;
use crate::error::GenesisResult;
use crate::laws::{LawDelta, PhysicalLaws};
use serde::{Deserialize, Serialize};

/// Workload observables that drive law rewriting.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkloadProfile {
    /// Mean residual of recent chronal solves.
    pub mean_residual: f64,
    /// Fraction of MultiWeighted / forked outcomes.
    pub fork_rate: f64,
    /// Holographic compression ratio in use.
    pub holo_compression: f64,
    /// Net thermodynamic work sign (− harvest, + dissipate).
    pub thermo_net_j: f64,
    /// Average agent correction rate.
    pub agent_correction_rate: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetaCompileReport {
    pub delta: LawDelta,
    pub epochs: usize,
    pub converged: bool,
    pub message: String,
}

/// Meta-compiler \(G\): maps workload observables → updated physical laws.
pub struct MetaCompiler {
    pub config: GenesisConfig,
}

impl MetaCompiler {
    pub fn new(config: GenesisConfig) -> Self {
        Self { config }
    }

    /// Single meta-epoch: propose new laws from workload.
    pub fn step(&self, laws: &PhysicalLaws, work: &WorkloadProfile) -> GenesisResult<PhysicalLaws> {
        let mut next = laws.clone();
        let lr = self.config.law_learn_rate;

        // Tighter Deutsch tolerance when residuals are already small.
        let target_tol = if work.mean_residual < 1e-8 {
            (laws.deutsch_tolerance * 0.5).max(self.config.deutsch_tol_min)
        } else if work.mean_residual > 1e-4 {
            (laws.deutsch_tolerance * 2.0).min(self.config.deutsch_tol_max)
        } else {
            laws.deutsch_tolerance
        };
        next.deutsch_tolerance = lerp(laws.deutsch_tolerance, target_tol, lr);

        // Faster chronal signalling when fork rate is high (need quicker consensus).
        let target_speed = (1.0 + 2.0 * work.fork_rate).clamp(
            self.config.signal_speed_min,
            self.config.signal_speed_max,
        );
        next.signal_speed = lerp(laws.signal_speed, target_speed, lr);

        // Boundary ratio tracks observed holographic compression needs.
        let target_ratio = work
            .holo_compression
            .clamp(self.config.boundary_ratio_min, self.config.boundary_ratio_max);
        next.boundary_ratio = lerp(laws.boundary_ratio, target_ratio, lr);

        // Manifold resolution increases with agent activity.
        let target_res = (1.0 + work.agent_correction_rate).clamp(0.25, 8.0);
        next.manifold_resolution = lerp(laws.manifold_resolution, target_res, lr);

        // Anderson β relaxes when thermo harvesting dominates (stable basin).
        let target_beta = if work.thermo_net_j < 0.0 { 1.0 } else { 0.7 };
        next.anderson_beta = lerp(laws.anderson_beta, target_beta, lr);

        next.clamp_to(&self.config)?;
        Ok(next)
    }

    /// Iterate meta-compilation until law fixed point or epoch budget exhausted.
    pub fn compile_to_fixed_point(
        &self,
        initial: PhysicalLaws,
        mut profile_fn: impl FnMut(&PhysicalLaws) -> WorkloadProfile,
    ) -> GenesisResult<MetaCompileReport> {
        let mut laws = initial;
        let mut epochs = 0;
        let mut last_delta = 0.0;
        for epoch in 0..self.config.max_meta_epochs {
            epochs = epoch + 1;
            let work = profile_fn(&laws);
            let next = self.step(&laws, &work)?;
            last_delta = laws.l2_distance(&next);
            let delta = LawDelta {
                before: laws.clone(),
                after: next.clone(),
                distance: last_delta,
            };
            laws = next;
            if last_delta < self.config.law_tolerance {
                return Ok(MetaCompileReport {
                    message: format!(
                        "genesis laws locked after {epochs} meta-epochs (Δ={last_delta:.3e})"
                    ),
                    delta,
                    epochs,
                    converged: true,
                });
            }
        }
        Ok(MetaCompileReport {
            delta: LawDelta {
                before: laws.clone(),
                after: laws,
                distance: last_delta,
            },
            epochs,
            converged: false,
            message: format!(
                "genesis meta-compile exhausted {epochs} epochs (Δ={last_delta:.3e})"
            ),
        })
    }
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}
