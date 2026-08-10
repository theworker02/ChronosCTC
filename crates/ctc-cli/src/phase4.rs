//! Phase 4 multiversal lifecycle.
//!
//! ```text
//! MultiWeighted solve ──► ledger.bifurcate ──► agents.explore
//!                                                     │
//!                                                     ▼
//!                              collapse.synthesize (Proof-of-Consistency)
//!                                                     │
//!                                                     ▼
//!                              primary thread cemented · losers pruned
//! ```

use crate::config::RuntimeConfig;
use ctc_agents::AgentFleet;
use ctc_collapse::CollapseEngine;
use ctc_dag::WorldlineDag;
use ctc_kernel::{ChronalKernel, ConvergenceClass, FnEvolution, NonlinearSystem, SolverConfig};
use ctc_ledger::{ForkCause, OmniversalLedger};
use nalgebra::DVector;
use std::sync::Arc;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MultiverseReport {
    pub bifurcation_children: usize,
    pub agents_deployed: usize,
    pub findings: usize,
    pub corrections: usize,
    pub winner: u64,
    pub primary_state: Vec<f64>,
    pub pruned: Vec<u64>,
    pub margin: f64,
    pub message: String,
}

/// End-to-end Phase-4 lifecycle on an identity CTC (continuum of fixed points).
pub fn run_multiverse_synthesis(runtime: &RuntimeConfig) -> Result<MultiverseReport, String> {
    // Identity map admits every point as a fixed point → MultiWeighted under multi-start.
    let evo = Arc::new(FnEvolution::new(1, |x: &DVector<f64>| x.clone()));
    let system =
        NonlinearSystem::new("multiverse_identity", evo, vec!["ρ".into()]).map_err(|e| e.to_string())?;

    let mut cfg = SolverConfig {
        num_restarts: 6,
        cluster_eps: 1e-3,
        domain_lo: 0.0,
        domain_hi: 1.0,
        ..runtime.solver.to_kernel_config()
    };
    cfg.tolerance = 1e-12;

    let solution = ChronalKernel::new(cfg)
        .solve(&system)
        .map_err(|e| e.to_string())?;
    if solution.class != ConvergenceClass::MultiWeighted {
        return Err(format!("expected MultiWeighted, got {:?}", solution.class));
    }

    let ledger = OmniversalLedger::new(runtime.ledger.clone());
    let root = ledger.bootstrap("production", WorldlineDag::new());
    let event = ledger
        .bifurcate_from_solution(root, &solution, ForkCause::MultiFixedPoint)
        .map_err(|e| e.to_string())?;

    let fleet = AgentFleet::new(runtime.agents.clone());
    fleet.deploy_triad(&event.children);
    let fleet_report = fleet.explore_all(&ledger).map_err(|e| e.to_string())?;

    let engine = CollapseEngine::new(runtime.collapse.clone());
    let synthesis = engine
        .synthesize(&ledger, Some(&fleet_report))
        .map_err(|e| e.to_string())?;

    let primary_state = ledger
        .fixed_point(ledger.primary())
        .ok_or("primary missing fixed point")?;

    Ok(MultiverseReport {
        bifurcation_children: event.children.len(),
        agents_deployed: fleet_report.agents_deployed,
        findings: fleet_report.findings,
        corrections: fleet_report.corrections_injected,
        winner: synthesis.winner,
        primary_state,
        pruned: synthesis.pruned,
        margin: synthesis.consensus.margin,
        message: synthesis.message,
    })
}
