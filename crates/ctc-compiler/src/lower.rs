use crate::ast::{ChronalModule, Expr};
use crate::error::{CompileError, CompileResult};
use crate::temporal::TemporalDependencyGraph;
use ctc_dag::{DependencyKind, LogicalAddr, NodeState, SpacetimeAddr, WorldlineDag};
use ctc_kernel::{FnEvolution, NonlinearSystem, SolverConfig};
use nalgebra::DVector;
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Artifact of lowering: kernel system + worldline fabric seed + solver hints.
#[derive(Clone, Debug)]
pub struct LoweringOutput {
    pub temporal: TemporalDependencyGraph,
    pub unknown_order: Vec<String>,
    pub solver_config: SolverConfig,
}

pub struct CompiledProgram {
    pub module_name: String,
    pub system: NonlinearSystem,
    pub dag: WorldlineDag,
    pub meta: LoweringOutput,
}

/// Lower a chronal module into a nonlinear system and seeded worldline DAG.
///
/// ## Mathematical form
///
/// Registers \(r_1,\ldots,r_n\) become unknowns \(x \in \mathbb{R}^n\). Each
/// `evolve` equation defines coordinate \(F_i(x)\). The kernel then solves
/// \(x = F(x)\).
pub fn lower(module: &ChronalModule) -> CompileResult<CompiledProgram> {
    let temporal = TemporalDependencyGraph::analyze(module)?;
    let order = temporal.allocation_order();
    if order.is_empty() {
        return Err(CompileError::Lowering("module has no registers".into()));
    }

    let mut index: FxHashMap<String, usize> = FxHashMap::default();
    let mut unknown_order = Vec::new();
    for (i, (name, _)) in order.iter().enumerate() {
        index.insert(name.clone(), i);
        unknown_order.push(name.clone());
    }

    let evolve_map: FxHashMap<&str, &Expr> = module
        .evolve_eqs()
        .into_iter()
        .map(|e| (e.target.as_str(), &e.body))
        .collect();

    for name in &unknown_order {
        if !evolve_map.contains_key(name.as_str()) {
            return Err(CompileError::MissingEvolution(name.clone()));
        }
    }

    // Capture owned equation bodies for the 'static evolution closure.
    let bodies: Vec<Expr> = unknown_order
        .iter()
        .map(|n| evolve_map[n.as_str()].clone())
        .collect();
    let index_for_eval = index.clone();
    let dim = bodies.len();

    let evolution = Arc::new(FnEvolution::new(dim, move |x: &DVector<f64>| {
        let mut out = DVector::zeros(dim);
        for (i, body) in bodies.iter().enumerate() {
            out[i] = eval_expr(body, x, &index_for_eval);
        }
        out
    }));

    let system = NonlinearSystem::new(module.name.clone(), evolution, unknown_order.clone())?;

    // Seed worldline DAG with spacetime cells and dependency edges.
    let mut dag = WorldlineDag::new();
    let mut node_ids = FxHashMap::default();
    for (i, (name, epoch)) in order.iter().enumerate() {
        let addr = SpacetimeAddr {
            address: LogicalAddr(i as u64),
            tau: *epoch,
        };
        let id = dag.allocate(addr, NodeState::scalar(0.5));
        node_ids.insert(name.clone(), id);
        // Keep name→addr recoverable via logical address = index.
        let _ = name;
    }

    for crossing in &temporal.crossings {
        let from = node_ids[&crossing.from];
        let to = node_ids[&crossing.to];
        let kind = if crossing.retrocausal {
            DependencyKind::Retrocausal
        } else {
            DependencyKind::Causal
        };
        dag.add_dependency(from, to, kind)
            .map_err(|e| CompileError::Lowering(e.to_string()))?;
    }

    // Algebraic couplings within an epoch from evolve RHS.
    for eq in module.evolve_eqs() {
        let target = node_ids[&eq.target];
        for dep in crate_expr_vars(&eq.body) {
            if dep == eq.target {
                continue;
            }
            if let Some(&from) = node_ids.get(dep) {
                let from_epoch = temporal.epochs[dep];
                let to_epoch = temporal.epochs[&eq.target];
                if from_epoch == to_epoch {
                    // Ignore duplicate algebraic edges.
                    let _ = dag.add_dependency(from, target, DependencyKind::Algebraic);
                }
            }
        }
    }

    let mut solver_config = SolverConfig::default();
    // CTC regions benefit from broader multi-start coverage.
    if !temporal.ctc_region.is_empty() {
        solver_config.num_restarts = solver_config.num_restarts.max(12);
        solver_config.anderson_m = solver_config.anderson_m.max(6);
    }

    Ok(CompiledProgram {
        module_name: module.name.clone(),
        system,
        dag,
        meta: LoweringOutput {
            temporal,
            unknown_order,
            solver_config,
        },
    })
}

fn eval_expr(expr: &Expr, x: &DVector<f64>, index: &FxHashMap<String, usize>) -> f64 {
    match expr {
        Expr::Const(c) => *c,
        Expr::Var(v) => x[index[v]],
        Expr::Add(a, b) => eval_expr(a, x, index) + eval_expr(b, x, index),
        Expr::Sub(a, b) => eval_expr(a, x, index) - eval_expr(b, x, index),
        Expr::Mul(a, b) => eval_expr(a, x, index) * eval_expr(b, x, index),
        Expr::Neg(a) => -eval_expr(a, x, index),
    }
}

fn crate_expr_vars(expr: &Expr) -> Vec<&str> {
    use Expr::*;
    match expr {
        Const(_) => vec![],
        Var(v) => vec![v.as_str()],
        Add(a, b) | Sub(a, b) | Mul(a, b) => {
            let mut v = crate_expr_vars(a);
            v.extend(crate_expr_vars(b));
            v
        }
        Neg(a) => crate_expr_vars(a),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_module;
    use ctc_kernel::{ChronalKernel, ConvergenceClass};

    #[test]
    fn lowers_and_converges_linear_ctc() {
        // Fixed point of:
        //   x = 0.5 x + 0.25 y
        //   y = 0.25 x + 0.5 y + 0.5
        // => 0.5x - 0.25y = 0
        //    -0.25x + 0.5y = 0.5
        // Solve: from first y = 2x; plug: -0.25x + x = 0.5 => 0.75x = 0.5 => x=2/3, y=4/3
        let src = r#"
            chronal reg x @ τ0;
            chronal reg y @ τ1;
            retrocausal { y -> x };
            evolve x = 0.5 * x + 0.25 * y;
            evolve y = 0.25 * x + 0.5 * y + 0.5;
        "#;
        let module = parse_module("linear_ctc", src).unwrap();
        let compiled = lower(&module).unwrap();
        assert!(!compiled.meta.temporal.ctc_region.is_empty());

        let kernel = ChronalKernel::new(compiled.meta.solver_config.clone());
        let sol = kernel.solve(&compiled.system).unwrap();
        assert_eq!(sol.class, ConvergenceClass::Unique);

        // Map back by unknown order.
        let mut vals = FxHashMap::default();
        for (name, v) in compiled.meta.unknown_order.iter().zip(sol.states[0].iter()) {
            vals.insert(name.as_str(), *v);
        }
        assert!((vals["x"] - 2.0 / 3.0).abs() < 1e-6);
        assert!((vals["y"] - 4.0 / 3.0).abs() < 1e-6);
    }
}
