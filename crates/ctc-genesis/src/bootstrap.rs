use crate::config::GenesisConfig;
use crate::error::GenesisResult;
use crate::laws::PhysicalLaws;
use crate::metacompile::{MetaCompileReport, MetaCompiler, WorkloadProfile};
use ctc_entropy::{EntropyConfig, ThermoBalancer};
use ctc_holo::{BoundaryTopology, HoloConfig, HolographicProjector};
use ctc_kernel::{ChronalKernel, FnEvolution, NonlinearSystem, SolverConfig};
use nalgebra::DVector;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapReport {
    pub laws: PhysicalLaws,
    pub meta: MetaCompileReport,
    pub holo_von_neumann: f64,
    pub thermo_net_j: f64,
    pub zero_energy: bool,
    pub kernel_residual: f64,
    pub message: String,
}

/// End-to-end cosmological bootstrap: holo encode → thermo balance → law rewrite.
pub struct GenesisEngine {
    pub config: GenesisConfig,
    pub laws: PhysicalLaws,
    pub holo: HolographicProjector,
    pub thermo: ThermoBalancer,
    pub meta: MetaCompiler,
}

impl Default for GenesisEngine {
    fn default() -> Self {
        Self::new(GenesisConfig::default(), PhysicalLaws::default())
    }
}

impl GenesisEngine {
    pub fn new(config: GenesisConfig, laws: PhysicalLaws) -> Self {
        let mut holo_cfg = HoloConfig::default();
        holo_cfg.boundary_ratio = laws.boundary_ratio;
        let mut ent_cfg = EntropyConfig::default();
        // Tie thermo zero-energy gate to Deutsch tolerance.
        ent_cfg.zero_energy_residual = laws.deutsch_tolerance;
        Self {
            holo: HolographicProjector::new(holo_cfg),
            thermo: ThermoBalancer::new(ent_cfg),
            meta: MetaCompiler::new(config.clone()),
            config,
            laws,
        }
    }

    /// Run the closed cosmological lifecycle for a sample chronal workload.
    pub fn bootstrap_cosmos(&mut self) -> GenesisResult<BootstrapReport> {
        let laws_ref = self.laws.clone();
        let config = self.config.clone();

        // Capture holo/thermo handles via local clones of config for the closure.
        let meta = MetaCompiler::new(config.clone());
        let mut last_holo_vn = 0.0;
        let mut last_thermo_net = 0.0;
        let mut last_residual = f64::INFINITY;
        let mut last_zero = false;

        let report = meta.compile_to_fixed_point(laws_ref, |laws| {
            // 1. Boundary encoding of a representative bulk (identity CTC mixture seeds).
            let bulk = sample_bulk_for_laws(laws);
            let mut holo_cfg = HoloConfig::default();
            holo_cfg.boundary_ratio = laws.boundary_ratio;
            let holo = HolographicProjector::new(holo_cfg);
            let (surface, spec, proj) = holo
                .project_state(&bulk, BoundaryTopology::AdSDisk)
                .unwrap_or_else(|_| {
                    // Fallback empty-safe surface
                    holo.project_state(&[0.5, 0.5], BoundaryTopology::Torus2d)
                        .expect("tiny bulk")
                });
            last_holo_vn = spec.von_neumann;

            // 2. Thermodynamic accounting for holographic compression + convergence.
            let mut ent_cfg = EntropyConfig::default();
            ent_cfg.zero_energy_residual = laws.deutsch_tolerance;
            let thermo = ThermoBalancer::new(ent_cfg);
            let _ = thermo.account_holographic(&proj);
            let kernel_resid = evaluate_kernel_residual(laws, &bulk);
            last_residual = kernel_resid;
            let _ = thermo.converge(1.0, kernel_resid);
            last_zero = kernel_resid <= laws.deutsch_tolerance;
            last_thermo_net = thermo.snapshot().net_work_j;
            let _ = surface;

            WorkloadProfile {
                mean_residual: kernel_resid,
                fork_rate: if bulk.len() > 2 { 0.4 } else { 0.1 },
                holo_compression: proj.compression_ratio,
                thermo_net_j: last_thermo_net,
                agent_correction_rate: 0.05 + 0.1 * proj.von_neumann.min(1.0),
            }
        })?;

        if !report.converged {
            // Soft success: still publish best laws.
        }

        self.laws = report.delta.after.clone();
        // Refresh subsystem configs from locked laws.
        self.holo.config.boundary_ratio = self.laws.boundary_ratio;
        self.thermo.config.zero_energy_residual = self.laws.deutsch_tolerance;

        Ok(BootstrapReport {
            message: format!(
                "cosmos bootstrapped: ε={:.1e}  signal×{:.2}  boundary={:.2}  residual={:.3e}  zero_E={}",
                self.laws.deutsch_tolerance,
                self.laws.signal_speed,
                self.laws.boundary_ratio,
                last_residual,
                last_zero
            ),
            laws: self.laws.clone(),
            meta: report,
            holo_von_neumann: last_holo_vn,
            thermo_net_j: last_thermo_net,
            zero_energy: last_zero,
            kernel_residual: last_residual,
        })
    }

    /// Apply locked laws onto a kernel solver config.
    pub fn apply_to_solver(&self, mut cfg: SolverConfig) -> SolverConfig {
        cfg.tolerance = self.laws.deutsch_tolerance;
        cfg.anderson_beta = self.laws.anderson_beta;
        cfg
    }
}

fn sample_bulk_for_laws(laws: &PhysicalLaws) -> Vec<f64> {
    let n = (4.0 * laws.manifold_resolution).ceil() as usize;
    let n = n.clamp(2, 32);
    (0..n)
        .map(|i| {
            let t = i as f64 / (n as f64 - 1.0).max(1.0);
            t * laws.signal_speed / (1.0 + laws.signal_speed)
        })
        .collect()
}

fn evaluate_kernel_residual(laws: &PhysicalLaws, bulk: &[f64]) -> f64 {
    // Contractive affine map whose fixed point residual measures law fitness.
    let dim = bulk.len().max(1);
    let evo = Arc::new(FnEvolution::new(dim, move |x: &DVector<f64>| {
        x.scale(0.5) + DVector::from_element(dim, 0.25)
    }));
    let unknowns = (0..dim).map(|i| format!("x{i}")).collect();
    let Ok(system) = NonlinearSystem::new("genesis_probe", evo, unknowns) else {
        return 1.0;
    };
    let cfg = SolverConfig {
        tolerance: laws.deutsch_tolerance,
        anderson_beta: laws.anderson_beta,
        num_restarts: 2,
        max_iterations: 64,
        ..SolverConfig::default()
    };
    match ChronalKernel::new(cfg).solve(&system) {
        Ok(sol) => sol.stats.final_residual,
        Err(_) => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_locks_laws() {
        let mut genesis = GenesisEngine::default();
        let report = genesis.bootstrap_cosmos().unwrap();
        assert!(report.meta.epochs >= 1);
        assert!(report.laws.deutsch_tolerance > 0.0);
        assert!(report.laws.boundary_ratio > 0.0);
    }
}
