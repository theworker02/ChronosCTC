//! Cronos-CTC demonstration driver.
//!
//! Runs three chronal scenarios end-to-end:
//! 1. **Consistent linear CTC** — unique Deutsch fixed point
//! 2. **Identity multi-attractor** — probability-weighted mixture
//! 3. **Paradoxical shift** — Paradox Pruner hard-abort path

use ctc_compiler::{lower, parse_module};
use ctc_dag::{DependencyKind, NodeState, SpacetimeAddr, WorldlineDag};
use ctc_kernel::{
    AffineEvolution, ChronalKernel, ConvergenceClass, FnEvolution, NonlinearSystem, SolverConfig,
};
use ctc_pruner::{ParadoxPruner, PruneAction, PrunerConfig};
use nalgebra::{DMatrix, DVector};
use std::sync::Arc;

fn main() {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║           Cronos-CTC Chronal Runtime v0.1.0              ║");
    println!("║   Closed Timelike Curve Fixed-Point Execution Engine     ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");

    scenario_consistent_ctc();
    println!();
    scenario_multi_weighted();
    println!();
    scenario_paradox();
}

fn scenario_consistent_ctc() {
    println!("── Scenario A: Consistent Linear CTC ──────────────────────");
    println!("  Deutsch condition: find x = F(x) for retrocausal block");
    println!("  x@τ0 = 0.5·x + 0.25·y");
    println!("  y@τ1 = 0.25·x + 0.5·y + 0.5");
    println!("  analytic fixed point: (x, y) = (2/3, 4/3)\n");

    let src = r#"
        chronal reg x @ τ0;
        chronal reg y @ τ1;
        retrocausal { y -> x };
        evolve x = 0.5 * x + 0.25 * y;
        evolve y = 0.25 * x + 0.5 * y + 0.5;
    "#;

    let module = parse_module("consistent_ctc", src).expect("parse");
    let mut compiled = lower(&module).expect("lower");
    let kernel = ChronalKernel::new(compiled.meta.solver_config.clone());
    let solution = kernel.solve(&compiled.system).expect("solve");

    let pruner = ParadoxPruner::new(PrunerConfig::default());
    let branch = pruner.branches.seed_root(&compiled.dag, 0.0);
    // Seed a stable parent so collapse has somewhere to go if needed.
    let _ = branch;

    let report = pruner
        .observe_solution(branch, &solution, &mut compiled.dag)
        .expect("prune observe");

    println!("  convergence class : {:?}", solution.class);
    for (i, (state, w)) in solution
        .states
        .iter()
        .zip(solution.weights.iter())
        .enumerate()
    {
        print!("  attractor[{i}] weight={w:.4}  values={{ ");
        for (name, v) in compiled.meta.unknown_order.iter().zip(state.iter()) {
            print!("{name}={v:.8} ");
        }
        println!("}}");
    }
    println!(
        "  solver stats      : iters={} residual={:.3e} restarts={}",
        solution.stats.iterations,
        solution.stats.final_residual,
        solution.stats.restarts_used
    );
    println!("  pruner action     : {:?}", report.action);
    println!(
        "  worldline edges   : {} retrocausal",
        compiled.dag.retrocausal_edges().len()
    );

    // Apply solution into the DAG fabric.
    apply_solution_to_dag(&mut compiled.dag, &compiled.meta.unknown_order, &solution);
    println!("  fabric locked     : {} spacetime cells", compiled.dag.len());
}

fn scenario_multi_weighted() {
    println!("── Scenario B: Multi-Attractor Identity CTC ───────────────");
    println!("  F(x) = x on [0,1] — continuum of fixed points");
    println!("  kernel clusters multi-start landings → Deutsch mixture\n");

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
    let wsum: f64 = solution.weights.iter().sum();
    println!("  Σ weights         : {wsum:.6}");
}

fn scenario_paradox() {
    println!("── Scenario C: Paradoxical Affine Shift ───────────────────");
    println!("  F(x) = x + 1  ⇒  no real fixed point (empty solution set)");
    println!("  expected: ConvergenceClass::Paradox → Pruner HardAbort\n");

    let evo = Arc::new(AffineEvolution {
        a: DMatrix::identity(1, 1),
        b: DVector::from_vec(vec![1.0]),
    });
    let system = NonlinearSystem::new("paradox_shift", evo, vec!["x".into()]).unwrap();
    let mut cfg = SolverConfig::default();
    cfg.max_iterations = 48;
    cfg.num_restarts = 4;
    cfg.tolerance = 1e-12;

    let solution = ChronalKernel::new(cfg).solve(&system).unwrap();

    let mut dag = WorldlineDag::new();
    let n0 = dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.0));
    let n1 = dag.allocate(SpacetimeAddr::new(0, 1), NodeState::scalar(1.0));
    dag.add_dependency(n1, n0, DependencyKind::Retrocausal)
        .unwrap();
    dag.retro_write(SpacetimeAddr::new(0, 1), Arc::from([2.0]))
        .unwrap();

    let pruner = ParadoxPruner::default();
    // Only one branch — paradox should hard-abort.
    let branch = pruner.branches.seed_root(&dag, 0.0);
    // Force the root into the solve observation as the failing branch.
    let report = pruner
        .observe_solution(branch, &solution, &mut dag)
        .unwrap();

    println!("  convergence class : {:?}", solution.class);
    println!(
        "  final residual     : {:.3e} (iters={})",
        solution.stats.final_residual, solution.stats.iterations
    );
    println!("  pruner action     : {:?}", report.action);
    println!("  message           : {}", report.message);
    assert_eq!(solution.class, ConvergenceClass::Paradox);
    assert_eq!(report.action, PruneAction::HardAbort);
    println!("  verdict           : timeline rejected (paradox pruned)");
}

fn apply_solution_to_dag(
    dag: &mut WorldlineDag,
    order: &[String],
    solution: &ctc_kernel::FixedPointSolution,
) {
    if solution.states.is_empty() {
        return;
    }
    // Lock the primary (highest-weight) attractor into the fabric.
    let (idx, _) = solution
        .weights
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .unwrap();
    let state = &solution.states[idx];
    for (i, _name) in order.iter().enumerate() {
        // Lowering assigned LogicalAddr(i) in allocation order.
        // Find node by scanning snapshot.
        if let Ok(snap) = dag.snapshot() {
            if let Some((addr, _, _)) = snap.nodes.get(i) {
                let _ = dag.retro_write(*addr, Arc::from([state[i]]));
            }
        }
    }
    let dirty = dag.dirty_nodes();
    dag.clear_dirty(&dirty);
}
