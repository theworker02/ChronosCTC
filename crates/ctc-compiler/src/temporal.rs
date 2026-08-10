use crate::ast::ChronalModule;
use crate::error::{CompileError, CompileResult};
use ctc_dag::Epoch;
use rustc_hash::{FxHashMap, FxHashSet};
use serde::{Deserialize, Serialize};

/// An edge whose endpoints lie in distinct epochs — the compiler's primary
/// optimization cue for worldline memory layout.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochCrossing {
    pub from: String,
    pub to: String,
    pub from_epoch: Epoch,
    pub to_epoch: Epoch,
    pub retrocausal: bool,
}

/// Temporal dependency graph extracted from a [`ChronalModule`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TemporalDependencyGraph {
    pub epochs: FxHashMap<String, Epoch>,
    pub crossings: Vec<EpochCrossing>,
    /// Registers participating in at least one retrocausal edge.
    pub ctc_region: FxHashSet<String>,
}

impl TemporalDependencyGraph {
    pub fn analyze(module: &ChronalModule) -> CompileResult<Self> {
        let mut epochs = FxHashMap::default();
        for reg in module.registers() {
            if epochs.insert(reg.name.clone(), reg.epoch).is_some() {
                return Err(CompileError::DuplicateRegister(reg.name.clone()));
            }
        }

        let mut crossings = Vec::new();
        let mut ctc_region = FxHashSet::default();

        // Explicit retrocausal wires.
        for (from, to) in module.retrocausal_edges() {
            let from_epoch = *epochs
                .get(from)
                .ok_or_else(|| CompileError::BadRetrocausalEdge {
                    from: from.into(),
                    to: to.into(),
                })?;
            let to_epoch = *epochs
                .get(to)
                .ok_or_else(|| CompileError::BadRetrocausalEdge {
                    from: from.into(),
                    to: to.into(),
                })?;
            ctc_region.insert(from.to_string());
            ctc_region.insert(to.to_string());
            crossings.push(EpochCrossing {
                from: from.to_string(),
                to: to.to_string(),
                from_epoch,
                to_epoch,
                retrocausal: true,
            });
        }

        // Infer crossings from evolve RHS variable references across epochs.
        for eq in module.evolve_eqs() {
            let target_epoch = *epochs
                .get(&eq.target)
                .ok_or_else(|| CompileError::UndefinedRegister(eq.target.clone()))?;
            for dep in expr_vars(&eq.body) {
                let dep_epoch = *epochs
                    .get(dep)
                    .ok_or_else(|| CompileError::UndefinedRegister(dep.to_string()))?;
                if dep_epoch != target_epoch {
                    let retrocausal = dep_epoch.0 > target_epoch.0;
                    if retrocausal {
                        ctc_region.insert(dep.to_string());
                        ctc_region.insert(eq.target.clone());
                    }
                    crossings.push(EpochCrossing {
                        from: dep.to_string(),
                        to: eq.target.clone(),
                        from_epoch: dep_epoch,
                        to_epoch: target_epoch,
                        retrocausal,
                    });
                }
            }
        }

        Ok(Self {
            epochs,
            crossings,
            ctc_region,
        })
    }

    /// Suggest worldline allocation order: earlier epochs first, CTC region
    /// registers packed contiguously for cache-local cascade propagation.
    pub fn allocation_order(&self) -> Vec<(String, Epoch)> {
        let mut regs: Vec<(String, Epoch)> = self
            .epochs
            .iter()
            .map(|(n, e)| (n.clone(), *e))
            .collect();
        regs.sort_by(|a, b| {
            let a_ctc = self.ctc_region.contains(&a.0);
            let b_ctc = self.ctc_region.contains(&b.0);
            b_ctc.cmp(&a_ctc).then(a.1.cmp(&b.1)).then(a.0.cmp(&b.0))
        });
        regs
    }
}

fn expr_vars(expr: &crate::ast::Expr) -> FxHashSet<&str> {
    use crate::ast::Expr::*;
    let mut set = FxHashSet::default();
    fn walk<'a>(e: &'a crate::ast::Expr, set: &mut FxHashSet<&'a str>) {
        match e {
            Const(_) => {}
            Var(v) => {
                set.insert(v.as_str());
            }
            Add(a, b) | Sub(a, b) | Mul(a, b) => {
                walk(a, set);
                walk(b, set);
            }
            Neg(a) => walk(a, set),
        }
    }
    walk(expr, &mut set);
    set
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_module;

    #[test]
    fn detects_retrocausal_region() {
        let m = parse_module(
            "t",
            r#"
            chronal reg x @ τ0;
            chronal reg y @ τ1;
            retrocausal { y -> x };
            evolve x = y;
            evolve y = 0.5;
        "#,
        )
        .unwrap();
        let g = TemporalDependencyGraph::analyze(&m).unwrap();
        assert!(g.ctc_region.contains("x"));
        assert!(g.ctc_region.contains("y"));
        assert!(g.crossings.iter().any(|c| c.retrocausal));
    }
}
