//! Phase 5 cosmological execution lifecycle.
//!
//! ```text
//! bulk DAG ──► ctc-holo boundary ──► ctc-entropy Landauer balance
//!                                            │
//!                                            ▼
//!                               ctc-genesis law fixed point Λ*
//!                                            │
//!                                            ▼
//!                          self-sustaining retrocausal cosmos
//! ```

use crate::config::RuntimeConfig;
use ctc_agents::AgentFleet;
use ctc_collapse::CollapseEngine;
use ctc_dag::WorldlineDag;
use ctc_entropy::ThermoBalancer;
use ctc_genesis::{GenesisEngine, PhysicalLaws};
use ctc_holo::{BoundaryTopology, HolographicProjector};
use ctc_kernel::{ChronalKernel, ConvergenceClass, FnEvolution, NonlinearSystem, SolverConfig};
use ctc_ledger::{ForkCause, OmniversalLedger};
use nalgebra::DVector;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct CosmosReport {
    pub holo_boundary_dim: usize,
    pub holo_von_neumann: f64,
    pub rt_entropy: f64,
    pub thermo_net_j: f64,
    pub zero_energy: bool,
    pub laws_deutsch: f64,
    pub laws_signal: f64,
    pub laws_boundary: f64,
    pub meta_epochs: usize,
    pub meta_converged: bool,
    pub multiverse_winner: Option<u64>,
    pub message: String,
}

/// Full Phase-5 lifecycle: holography → thermodynamics → genesis bootstrap,
/// with an optional multiversal collapse feeding the thermo ledger.
pub fn run_cosmological_lifecycle(runtime: &RuntimeConfig) -> Result<CosmosReport, String> {
    // ── 1. Boundary encoding of a high-dimensional bulk ───────────────
    let holo = HolographicProjector::new(runtime.holo.clone());
    let bulk: Vec<f64> = (0..24).map(|i| i as f64 / 23.0).collect();
    let (surface, spec, proj) = holo
        .project_state(&bulk, BoundaryTopology::AdSDisk)
        .map_err(|e| e.to_string())?;

    // ── 2. Thermodynamic equilibrium accounting ───────────────────────
    let thermo = ThermoBalancer::new(runtime.entropy.clone());
    let _ = thermo
        .account_holographic(&proj)
        .map_err(|e| e.to_string())?;

    // Multiversal fork → collapse → thermo prune costs
    let mut multiverse_winner = None;
    {
        let evo = Arc::new(FnEvolution::new(1, |x: &DVector<f64>| x.clone()));
        let system = NonlinearSystem::new("cosmos_id", evo, vec!["ρ".into()])
            .map_err(|e| e.to_string())?;
        let sol = ChronalKernel::new(SolverConfig {
            num_restarts: 4,
            cluster_eps: 1e-3,
            domain_lo: 0.0,
            domain_hi: 1.0,
            ..runtime.solver.to_kernel_config()
        })
        .solve(&system)
        .map_err(|e| e.to_string())?;

        if sol.class == ConvergenceClass::MultiWeighted {
            let ledger = OmniversalLedger::new(runtime.ledger.clone());
            let root = ledger.bootstrap("cosmos", WorldlineDag::new());
            let event = ledger
                .bifurcate_from_solution(root, &sol, ForkCause::MultiFixedPoint)
                .map_err(|e| e.to_string())?;
            let fleet = AgentFleet::new(runtime.agents.clone());
            fleet.deploy_triad(&event.children);
            let fleet_report = fleet.explore_all(&ledger).map_err(|e| e.to_string())?;
            let engine = CollapseEngine::new(runtime.collapse.clone());
            let synthesis = engine
                .synthesize(&ledger, Some(&fleet_report))
                .map_err(|e| e.to_string())?;
            multiverse_winner = Some(synthesis.winner);
            let _ = thermo
                .account_collapse(&synthesis)
                .map_err(|e| e.to_string())?;
        }
    }

    // Boundary fixed-point solves on the entanglement kernel (near-zero latency path).
    let (lifted, _) = holo
        .boundary_fixed_point(&bulk)
        .map_err(|e| e.to_string())?;
    // Lossy AdS/CFT encoding cannot recover the exact bulk; measure fidelity
    // against the original projection source, not the boundary lift.
    let recon = holo
        .reconstruct(&surface, Some(&bulk))
        .map_err(|e| e.to_string())?;
    debug_assert_eq!(lifted.len(), bulk.len());
    let _ = thermo.converge(1.0, proj.von_neumann.min(1e-6));
    let thermo_snap = thermo.snapshot();

    // ── 3. Reality compilation (Genesis law fixed point) ──────────────
    let mut genesis = GenesisEngine::new(runtime.genesis.clone(), PhysicalLaws {
        boundary_ratio: runtime.holo.boundary_ratio,
        deutsch_tolerance: runtime.solver.tolerance,
        ..PhysicalLaws::default()
    });
    let boot = genesis.bootstrap_cosmos().map_err(|e| e.to_string())?;

    let zero_energy = boot.zero_energy || boot.kernel_residual <= boot.laws.deutsch_tolerance;

    Ok(CosmosReport {
        message: format!(
            "{} | holo S_EE={:.4} RT={:.4} recon={:.3e} | thermo net={:.3e} J",
            boot.message,
            spec.von_neumann,
            proj.rt_entropy,
            recon.residual,
            thermo_snap.net_work_j
        ),
        holo_boundary_dim: surface.dim,
        holo_von_neumann: spec.von_neumann,
        rt_entropy: proj.rt_entropy,
        thermo_net_j: thermo_snap.net_work_j,
        zero_energy,
        laws_deutsch: boot.laws.deutsch_tolerance,
        laws_signal: boot.laws.signal_speed,
        laws_boundary: boot.laws.boundary_ratio,
        meta_epochs: boot.meta.epochs,
        meta_converged: boot.meta.converged,
        multiverse_winner,
    })
}
