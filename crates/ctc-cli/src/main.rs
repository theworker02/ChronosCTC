//! Cronos-CTC Phase 3 demonstration driver.
//!
//! Phases 1–2 remain available; Phase 3 adds the inter-epoch lifecycle:
//! 1. Future State Resolution
//! 2. Retrocausal Packaging (`ctc-signal`)
//! 3. Backward Injection / Mesh Relay (`ctc-mesh`)
//! 4. Pre-Cognitive Collapse (`ctc-oracle`)

mod config;
mod phase2;
mod phase3;

use ctc_compiler::{lower, parse_module};
use ctc_dag::{DependencyKind, NodeState, SpacetimeAddr, WorldlineDag};
use ctc_kernel::{AffineEvolution, ChronalKernel, ConvergenceClass, NonlinearSystem};
use ctc_pruner::{ParadoxPruner, PruneAction};
use nalgebra::{DMatrix, DVector};
use phase2::ConvergenceObservationLoop;
use std::sync::Arc;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         Cronos-CTC Chronal Runtime v0.3.0 (Phase 3)      ║");
    println!("║     Signal · Oracle · Mesh — Inter-Epoch Teleportation   ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    let runtime = config::load_runtime_config();
    println!(
        "config: signal.binding={}  oracle.timeout={}ms  mesh.hops={}µs\n",
        runtime.signal.require_binding,
        runtime.oracle.timeout_ms,
        runtime.mesh.hop_latency_us
    );

    // ── Phase 3 primary demos ──────────────────────────────────────────
    phase3_local_teleport(&runtime);
    println!();
    phase3_mesh_teleport(&runtime);
    println!();
    phase3_deutsch_gate(&runtime);
    println!();

    // ── Phase 2 regression smoke ───────────────────────────────────────
    phase2_consistent_ctc(&runtime);
    println!();
    phase2_paradox_reconverge(&runtime);
}

fn phase3_local_teleport(runtime: &config::RuntimeConfig) {
    println!("── Phase-3: Local Retrocausal Teleport ────────────────────");
    println!("  τ4 resolve → signal package → oracle collapse @ τ0\n");

    let report = phase3::run_local_teleport(runtime).expect("local teleport");
    println!("  {}", report.message);
    println!("  packet_id         : {}", report.packet_id);
    println!("  cycles skipped    : {}", report.cycles_saved);
    println!("  injected @ τ0     : {:.8}", report.injected_value);
    println!(
        "  cascade invalidated: {}",
        report.transmit.injection.cascade_invalidated
    );
}

fn phase3_mesh_teleport(runtime: &config::RuntimeConfig) {
    println!("── Phase-3: Distributed Mesh Entanglement ─────────────────");
    println!("  solver-Ω @ τ12 ══E══► worker-α @ τ0 (pre-cognitive)\n");

    let report = phase3::run_mesh_teleport(runtime).expect("mesh teleport");
    println!("  {}", report.message);
    println!("  entanglement      : E{}", report.entanglement);
    println!("  route             : {} → {}", report.from, report.to);
    println!("  cycles skipped    : {}", report.cycles_saved);
    println!("  injected hparam   : {:.9}", report.injected_value);
}

fn phase3_deutsch_gate(runtime: &config::RuntimeConfig) {
    println!("── Phase-3: Strict Deutsch-Gated Injection ────────────────");
    println!("  only fixed-point payloads may cross into the past\n");

    let msg = phase3::run_deutsch_gated_teleport(runtime).expect("deutsch gate");
    println!("  {msg}");
}

fn phase2_consistent_ctc(runtime: &config::RuntimeConfig) {
    println!("── Phase-2 Smoke: Consistent Linear CTC ───────────────────");

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
    let report = loop_engine.run(compiled).expect("phase2 loop");

    println!(
        "  device={:?}  class={:?}  residual={:.3e}",
        report.device_kind, report.solution.class, report.solution.stats.final_residual
    );
}

fn phase2_paradox_reconverge(runtime: &config::RuntimeConfig) {
    println!("── Phase-2 Smoke: Paradox → GC ─────────────────────────────");

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
    dag.add_dependency(n1, n0, DependencyKind::Retrocausal)
        .unwrap();

    let pruner = ParadoxPruner::new(runtime.pruner.clone());
    let root = pruner.branches.seed_root(&dag, 1e-12);
    let child = pruner.branches.fork(root, &dag).unwrap();
    let report = pruner
        .observe_solution(child, &solution, &mut dag)
        .unwrap();

    let mut gc = ctc_gc::TimelineGc::new(runtime.gc.clone());
    let gc_report = gc
        .force_reconverge_cleanup(&mut dag, &pruner.branches)
        .unwrap();

    println!(
        "  class={:?}  pruner={:?}  gc_checkpoints={}",
        solution.class,
        report.action,
        gc_report.checkpoint_ids.len()
    );
    assert_eq!(solution.class, ConvergenceClass::Paradox);
    assert!(matches!(
        report.action,
        PruneAction::Collapse { .. } | PruneAction::HardAbort | PruneAction::Invalidate
    ));
}
