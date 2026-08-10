use crate::branch::{BranchId, BranchManager};
use crate::error::{PrunerError, PrunerResult};
use ctc_dag::WorldlineDag;
use ctc_kernel::{ConvergenceClass, FixedPointSolution, ResidualMonitor};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrunerConfig {
    pub residual_ceiling: f64,
    pub auto_collapse: bool,
}

impl Default for PrunerConfig {
    fn default() -> Self {
        Self {
            residual_ceiling: 1e3,
            auto_collapse: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PruneAction {
    /// Residual healthy — no intervention.
    None,
    /// Branch invalidated; awaiting collapse.
    Invalidate,
    /// Timeline collapsed onto a stable alternative.
    Collapse { onto: u64 },
    /// Paradox with no alternative — hard abort signal.
    HardAbort,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PruneReport {
    pub action: PruneAction,
    pub branch: u64,
    pub residual_norm: f64,
    pub pruned_nodes: Vec<u64>,
    pub message: String,
}

/// Paradox Pruner daemon — invoke after each kernel solve (or mid-iteration).
pub struct ParadoxPruner {
    pub config: PrunerConfig,
    pub branches: BranchManager,
}

impl Default for ParadoxPruner {
    fn default() -> Self {
        Self::new(PrunerConfig::default())
    }
}

impl ParadoxPruner {
    pub fn new(config: PrunerConfig) -> Self {
        Self {
            config,
            branches: BranchManager::new(),
        }
    }

    /// Observe a completed kernel solution and act if paradoxical.
    pub fn observe_solution(
        &self,
        branch: BranchId,
        solution: &FixedPointSolution,
        dag: &mut WorldlineDag,
    ) -> PrunerResult<PruneReport> {
        self.branches.update_status(
            branch,
            solution.class.clone(),
            solution.stats.final_residual,
        )?;

        match solution.class {
            ConvergenceClass::Unique | ConvergenceClass::MultiWeighted => Ok(PruneReport {
                action: PruneAction::None,
                branch: branch.0,
                residual_norm: solution.stats.final_residual,
                pruned_nodes: vec![],
                message: format!(
                    "worldline consistent ({:?}); residual={:.3e}",
                    solution.class, solution.stats.final_residual
                ),
            }),
            ConvergenceClass::Paradox => self.prune_paradox(branch, solution.stats.final_residual, dag),
        }
    }

    /// Mid-iteration residual gate (call from solver hooks / outer loops).
    pub fn observe_residual(
        &self,
        branch: BranchId,
        monitor: &ResidualMonitor,
        dag: &mut WorldlineDag,
    ) -> PrunerResult<PruneReport> {
        let norm = monitor.last_norm();
        if norm > self.config.residual_ceiling || monitor.diverging() {
            self.branches
                .update_status(branch, ConvergenceClass::Paradox, norm)?;
            return self.prune_paradox(branch, norm, dag);
        }
        Ok(PruneReport {
            action: PruneAction::None,
            branch: branch.0,
            residual_norm: norm,
            pruned_nodes: vec![],
            message: format!("residual within gate: {norm:.3e}"),
        })
    }

    fn prune_paradox(
        &self,
        branch: BranchId,
        residual_norm: f64,
        dag: &mut WorldlineDag,
    ) -> PrunerResult<PruneReport> {
        // Rollback: prune all dirty nodes on the failing branch.
        let dirty = dag.dirty_nodes();
        let pruned_ids: Vec<u64> = dirty.iter().map(|n| n.0).collect();
        dag.prune_nodes(&dirty);
        self.branches.invalidate(branch)?;

        if !self.config.auto_collapse {
            return Ok(PruneReport {
                action: PruneAction::Invalidate,
                branch: branch.0,
                residual_norm,
                pruned_nodes: pruned_ids,
                message: "paradox detected; branch invalidated (auto-collapse disabled)".into(),
            });
        }

        match self.branches.nearest_stable(branch) {
            Ok(alt) => {
                let collapsed = self.branches.collapse_to(alt)?;
                // Restore checkpoint values onto surviving (non-pruned) addresses
                // where possible — re-allocate sealed-safe cells.
                restore_checkpoint(dag, &collapsed.checkpoint);
                Ok(PruneReport {
                    action: PruneAction::Collapse { onto: alt.0 },
                    branch: branch.0,
                    residual_norm,
                    pruned_nodes: pruned_ids,
                    message: format!(
                        "paradox pruned; timeline collapsed onto branch {}",
                        alt.0
                    ),
                })
            }
            Err(PrunerError::NoStableAlternative) => Ok(PruneReport {
                action: PruneAction::HardAbort,
                branch: branch.0,
                residual_norm,
                pruned_nodes: pruned_ids,
                message: "paradox with empty stable branch set — hard abort".into(),
            }),
            Err(e) => Err(e),
        }
    }
}

fn restore_checkpoint(dag: &mut WorldlineDag, checkpoint: &[(u64, i64, Vec<f64>)]) {
    use ctc_dag::{LogicalAddr, NodeState, SpacetimeAddr};
    use std::sync::Arc;
    for (addr, tau, vals) in checkpoint {
        let spa = SpacetimeAddr {
            address: LogicalAddr(*addr),
            tau: ctc_dag::Epoch(*tau),
        };
        let state = NodeState {
            value: Arc::from(vals.as_slice()),
            weight: 1.0,
            revision: 0,
            pruned: false,
        };
        // allocate() revises unsealed cells or creates missing ones; clear prune flag.
        let id = dag.allocate(spa, state);
        if let Some(n) = dag.node_mut(id) {
            n.state.pruned = false;
            n.dirty = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_dag::{NodeState, SpacetimeAddr};
    use ctc_kernel::SolverStats;
    use std::sync::Arc;

    #[test]
    fn paradox_hard_aborts_without_alternative() {
        let pruner = ParadoxPruner::default();
        let mut dag = WorldlineDag::new();
        let _ = dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.0));
        let branch = pruner.branches.seed_root(&dag, 0.0);
        // Mark dirty so prune has work.
        dag.retro_write(SpacetimeAddr::new(0, 0), Arc::from([1.0]))
            .unwrap();

        let sol = FixedPointSolution {
            class: ConvergenceClass::Paradox,
            states: vec![],
            weights: vec![],
            stats: SolverStats {
                iterations: 10,
                final_residual: 1e5,
                restarts_used: 4,
                fixed_points_found: 0,
            },
        };
        let report = pruner.observe_solution(branch, &sol, &mut dag).unwrap();
        assert_eq!(report.action, PruneAction::HardAbort);
    }

    #[test]
    fn collapse_onto_stable_sibling() {
        let pruner = ParadoxPruner::default();
        let mut dag = WorldlineDag::new();
        let _ = dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.0));
        let root = pruner.branches.seed_root(&dag, 1e-12);
        let child = pruner.branches.fork(root, &dag).unwrap();

        let sol = FixedPointSolution {
            class: ConvergenceClass::Paradox,
            states: vec![],
            weights: vec![],
            stats: SolverStats {
                iterations: 8,
                final_residual: 42.0,
                restarts_used: 2,
                fixed_points_found: 0,
            },
        };
        let report = pruner.observe_solution(child, &sol, &mut dag).unwrap();
        assert_eq!(report.action, PruneAction::Collapse { onto: root.0 });
        assert_eq!(pruner.branches.active(), Some(root));
    }
}
