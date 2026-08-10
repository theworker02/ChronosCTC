//! Cronos-CTC Phase 2 demonstration driver.
//!
//! Implements the **convergence-observation loop**:
//! 1. State Ingestion — compiler emits retrocausal dependency graph
//! 2. Parallel Exploration — bridge dispatches \(U(\rho)\) across devices
//! 3. Real-Time Telemetry — inspector streams residuals / manifold slices
//! 4. Automated Pruning — GC culls toxic branches and forces re-convergence

mod config;
mod phase2;

use ctc_compiler::{lower, parse_module};
use ctc_dag::{DependencyKind, NodeState, SpacetimeAddr, WorldlineDag};
use ctc_kernel::{
    AffineEvolution, ChronalKernel, ConvergenceClass, FnEvolution, NonlinearSystem, SolverConfig,
};
use ctc_pruner::{ParadoxPruner, PruneAction};
use nalgebra::{DMatrix, DVector};
use phase2::ConvergenceObservationLoop;
use std::sync::Arc;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         Cronos-CTC Chronal Runtime v0.2.0 (Phase 2)      ║");
    println!("║  Inspector · Bridge · Entropy GC · Convergence Loop      ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let runtime = config::load_runtime_config();
    println!(
        "config: solver.tol={:.1e}  bridge.ctc_pref[0]={:?}  gc.amplitude_floor={:.1e}\n",
        runtime.solver.tolerance,
        runtime.bridge.ctc_preference.first(),
        runtime.gc.amplitude_floor
    );

    phase2_consistent_ctc(&runtime);
    println!();
    scenario_multi_weighted();
    println!();
    phase2_paradox_reconverge(&runtime);
}

fn phase2_consistent_ctc(runtime: &config::RuntimeConfig) {
    println!("── Phase-2 Loop: Consistent Linear CTC ────────────────────");
    println!("  ingest → bridge(FPGA) → inspector telemetry → GC\n");

    let src = r#"
        chronal reg x @ τ0;
        chronal reg y @ τ1;
        retrocausal { y -> x };
        evolve x = 0.5 * x + 0.25 * y;
        evolve y = 0.25 * x + 0.5 * y + 0.5;
    "#;

    let module = parse_module("consistent_ctc", src).expect("parse");
    let compiled = lower(&module).expect("lower");

    let mut loop_engine = ConvergenceObservationLoop::from_runtime(runtime);
    let report = loop_engine
        .run(compiled)
        .expect("phase2 loop");

    println!("{}", report.manifold_ascii);
    println!("  {}", report.sparkline);
    println!("  device            : {:?} ({})", report.device_kind, report.device_id);
    println!("  block class       : {:?}", report.block_class);
    println!("  convergence class : {:?}", report.solution.class);
    for (i, (state, w)) in report
        .solution
        .states
        .iter()
        .zip(report.solution.weights.iter())
        .enumerate()
    {
        print!("  attractor[{i}] P={w:.4}  {{ ");
        for (name, v) in report.unknown_order.iter().zip(state.iter()) {
            print!("{name}={v:.8} ");
        }
        println!("}}");
    }
    if !report.hotspots.is_empty() {
        println!("  hotspots          :");
        for h in &report.hotspots {
            println!(
                "    component {} ({})  r={:.3e}  σ={:.2}",
                h.component,
                h.label.as_deref().unwrap_or("?"),
                h.value,
                h.sigma_ratio
            );
        }
    } else {
        println!("  hotspots          : (none at lock)");
    }
    println!(
        "  GC                : {}",
        report.gc_message.as_deref().unwrap_or("(idle)")
    );
    println!("  pruner            : {:?}", report.prune_action);
    println!(
        "  residual trend    : {} samples, final={:.3e}, reconverge={}",
        report.telemetry_samples,
        report.solution.stats.final_residual,
        report.reconverge_attempts
    );
}

fn scenario_multi_weighted() {
    println!("── Scenario B: Multi-Attractor Identity CTC ───────────────");
    println!("  F(x) = x on [0,1] — Deutsch mixture via multi-start\n");

    let evo = Arc::new(FnEvolution::new(1, |x| x.clone()));
    let system = NonlinearSystem::new("identity_ctc", evo, vec!["ρ".into()]).unwrap();
    let mut cfg = SolverConfig::default();
    cfg.num_restarts = 6;
    cfg.cluster_eps = 1e-3;
    cfg.domain_lo = 0.0;
    cfg.domain_hi = 1.0;

    let solution = ChronalKernel::new(cfg).solve(&system).unwrap();
    assert_eq!(solution.class, ConvergenceClass::MultiWeighted);

    println!("  convergence class : {:?}", solution.class);
    for (i, (state, w)) in solution
        .states
        .iter()
        .zip(solution.weights.iter())
        .enumerate()
    {
        println!(
            "  mixture component[{i}]: ρ={:.6}  P={:.4}",
            state[0], w
        );
    }
}

fn phase2_paradox_reconverge(runtime: &config::RuntimeConfig) {
    println!("── Phase-2 Loop: Paradox → GC Re-converge ─────────────────");
    println!("  F(x)=x+1 has no fixed point; GC prunes toxic timeline\n");

    let evo = Arc::new(AffineEvolution {
        a: DMatrix::identity(1, 1),
        b: DVector::from_vec(vec![1.0]),
    });
    let system = NonlinearSystem::new("paradox_shift", evo, vec!["x".into()]).unwrap();
    let mut cfg = runtime.solver.to_kernel_config();
    cfg.max_iterations = 48;
    cfg.num_restarts = 4;

    let solution = ChronalKernel::new(cfg).solve(&system).unwrap();

    let mut dag = WorldlineDag::new();
    let n0 = dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.0));
    let n1 = dag.allocate(SpacetimeAddr::new(0, 1), NodeState::scalar(1.0));
    let n_hist = dag.allocate(SpacetimeAddr::new(1, -3), NodeState::scalar(0.9));
    dag.add_dependency(n1, n0, DependencyKind::Retrocausal)
        .unwrap();
    dag.retro_write(SpacetimeAddr::new(0, 1), Arc::from([2.0]))
        .unwrap();
    let _ = n_hist;

    let pruner = ParadoxPruner::new(runtime.pruner.clone());
    let root = pruner.branches.seed_root(&dag, 1e-12);
    // Fork a speculative child that will absorb the paradox.
    let child = pruner.branches.fork(root, &dag).unwrap();
    let report = pruner
        .observe_solution(child, &solution, &mut dag)
        .unwrap();

    let mut gc = ctc_gc::TimelineGc::new(runtime.gc.clone());
    let gc_report = gc
        .force_reconverge_cleanup(&mut dag, &pruner.branches)
        .unwrap();

    println!("  convergence class : {:?}", solution.class);
    println!("  pruner action     : {:?}", report.action);
    println!("  active branch     : {:?}", pruner.branches.active());
    println!("  GC                : {}", gc_report.message);
    println!(
        "  checkpoints       : {} (compressed {} B)",
        gc_report.checkpoint_ids.len(),
        gc.checkpoints.total_compressed_bytes()
    );
    assert_eq!(solution.class, ConvergenceClass::Paradox);
    assert!(matches!(
        report.action,
        PruneAction::Collapse { .. } | PruneAction::HardAbort | PruneAction::Invalidate
    ));
    println!("  verdict           : toxic timeline pruned; manifold re-convergence armed");
}
