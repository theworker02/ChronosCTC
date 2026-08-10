//! Cronos-CTC Phase 4 demonstration driver.
//!
//! Multiversal lifecycle:
//! 1. Bifurcation — MultiWeighted fixed points fork parallel universes
//! 2. Autonomous Navigation — chronal agents probe each branch
//! 3. Consensus Evaluation — Proof-of-Consistency scores utility
//! 4. Reality Synthesis — winner merges into primary; losers pruned

mod config;
mod phase2;
mod phase3;
mod phase4;

use ctc_compiler::{lower, parse_module};
use ctc_dag::{DependencyKind, NodeState, SpacetimeAddr, WorldlineDag};
use ctc_kernel::{AffineEvolution, ChronalKernel, ConvergenceClass, NonlinearSystem};
use ctc_pruner::{ParadoxPruner, PruneAction};
use nalgebra::{DMatrix, DVector};
use phase2::ConvergenceObservationLoop;
use std::sync::Arc;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         Cronos-CTC Chronal Runtime v0.4.0 (Phase 4)      ║");
    println!("║   Ledger · Agents · Collapse — Multiversal Consensus     ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let runtime = config::load_runtime_config();
    println!(
        "config: collapse.margin={:.3}  agents.subopt_w={:.3}  ledger.max_live={}\n",
        runtime.collapse.consensus_margin,
        runtime.agents.suboptimal_weight,
        runtime.ledger.max_live_universes
    );

    phase4_multiverse(&runtime);
    println!();
    phase3_local_teleport(&runtime);
    println!();
    phase2_smoke(&runtime);
}

fn phase4_multiverse(runtime: &config::RuntimeConfig) {
    println!("── Phase-4: Multiversal Bifurcation → Collapse ────────────");
    println!("  MultiWeighted identity CTC → ledger forks → agent triad → PoC collapse\n");

    let report = phase4::run_multiverse_synthesis(runtime).expect("multiverse");
    println!("  {}", report.message);
    println!("  child universes   : {}", report.bifurcation_children);
    println!("  agents deployed   : {}", report.agents_deployed);
    println!("  probe findings    : {}", report.findings);
    println!("  corrections       : {}", report.corrections);
    println!("  winner universe   : U{}", report.winner);
    println!("  consensus margin  : {:.4}", report.margin);
    println!("  primary state     : {:?}", report.primary_state);
    println!("  pruned universes  : {:?}", report.pruned);
}

fn phase3_local_teleport(runtime: &config::RuntimeConfig) {
    println!("── Phase-3 Smoke: Local Teleport ──────────────────────────");
    let report = phase3::run_local_teleport(runtime).expect("teleport");
    println!(
        "  cycles_skipped={}  injected={:.6}",
        report.cycles_saved, report.injected_value
    );
}

fn phase2_smoke(runtime: &config::RuntimeConfig) {
    println!("── Phase-2 Smoke: Bridge CTC + Paradox GC ─────────────────");

    let src = r#"
        chronal reg x @ τ0;
        chronal reg y @ τ1;
        retrocausal { y -> x };
        evolve x = 0.5 * x + 0.25 * y;
        evolve y = 0.25 * x + 0.5 * y + 0.5;
    "#;
    let compiled = lower(&parse_module("ctc", src).unwrap()).unwrap();
    let report = ConvergenceObservationLoop::from_runtime(runtime)
        .run(compiled)
        .unwrap();
    println!(
        "  bridge device={:?} class={:?}",
        report.device_kind, report.solution.class
    );

    let evo = Arc::new(AffineEvolution {
        a: DMatrix::identity(1, 1),
        b: DVector::from_vec(vec![1.0]),
    });
    let system = NonlinearSystem::new("paradox", evo, vec!["x".into()]).unwrap();
    let mut cfg = runtime.solver.to_kernel_config();
    cfg.max_iterations = 32;
    cfg.num_restarts = 3;
    let solution = ChronalKernel::new(cfg).solve(&system).unwrap();
    let mut dag = WorldlineDag::new();
    let n0 = dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.0));
    let n1 = dag.allocate(SpacetimeAddr::new(0, 1), NodeState::scalar(1.0));
    dag.add_dependency(n1, n0, DependencyKind::Retrocausal)
        .unwrap();
    let pruner = ParadoxPruner::new(runtime.pruner.clone());
    let root = pruner.branches.seed_root(&dag, 0.0);
    let child = pruner.branches.fork(root, &dag).unwrap();
    let preport = pruner
        .observe_solution(child, &solution, &mut dag)
        .unwrap();
    println!(
        "  paradox class={:?} pruner={:?}",
        solution.class, preport.action
    );
    assert_eq!(solution.class, ConvergenceClass::Paradox);
    assert!(matches!(
        preport.action,
        PruneAction::Collapse { .. } | PruneAction::HardAbort | PruneAction::Invalidate
    ));
}
