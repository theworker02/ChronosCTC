use crate::bifurcation::{BifurcationEvent, BifurcationId, ForkCause};
use crate::config::LedgerConfig;
use crate::error::{LedgerError, LedgerResult};
use crate::universe::{UniverseBranch, UniverseId, UniverseStatus};
use ctc_dag::{NodeState, SpacetimeAddr, WorldlineDag};
use ctc_kernel::{ConvergenceClass, FixedPointSolution};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LedgerSnapshot {
    pub primary: u64,
    pub live_universes: usize,
    pub total_bifurcations: usize,
    pub sealed: bool,
    pub branches: Vec<(u64, f64, UniverseStatus, String)>,
}

/// Append-only omniversal ledger of reality branches.
pub struct OmniversalLedger {
    pub config: LedgerConfig,
    next_uid: RwLock<u64>,
    next_bid: RwLock<u64>,
    universes: RwLock<FxHashMap<u64, UniverseBranch>>,
    events: RwLock<Vec<BifurcationEvent>>,
    primary: RwLock<UniverseId>,
    sealed: RwLock<bool>,
}

impl Default for OmniversalLedger {
    fn default() -> Self {
        Self::new(LedgerConfig::default())
    }
}

impl OmniversalLedger {
    pub fn new(config: LedgerConfig) -> Self {
        Self {
            config,
            next_uid: RwLock::new(1),
            next_bid: RwLock::new(1),
            universes: RwLock::new(FxHashMap::default()),
            events: RwLock::new(Vec::new()),
            primary: RwLock::new(UniverseId(0)),
            sealed: RwLock::new(false),
        }
    }

    /// Bootstrap the primary universe from an existing worldline fabric.
    pub fn bootstrap(&self, label: impl Into<String>, dag: WorldlineDag) -> UniverseId {
        let id = self.alloc_uid();
        let mut root = UniverseBranch::root(label, dag);
        root.id = id;
        self.universes.write().insert(id.0, root);
        *self.primary.write() = id;
        id
    }

    pub fn primary(&self) -> UniverseId {
        *self.primary.read()
    }

    pub fn set_primary(&self, id: UniverseId) -> LedgerResult<()> {
        let u = self.universes.read();
        let b = u.get(&id.0).ok_or(LedgerError::UnknownUniverse(id.0))?;
        if !b.is_live() {
            return Err(LedgerError::InactiveParent(id.0));
        }
        *self.primary.write() = id;
        Ok(())
    }

    pub fn weight(&self, id: UniverseId) -> Option<f64> {
        self.universes.read().get(&id.0).map(|u| u.weight)
    }

    pub fn status(&self, id: UniverseId) -> Option<UniverseStatus> {
        self.universes.read().get(&id.0).map(|u| u.status)
    }

    pub fn fixed_point(&self, id: UniverseId) -> Option<Vec<f64>> {
        self.universes
            .read()
            .get(&id.0)
            .map(|u| u.fixed_point.clone())
    }

    pub fn label(&self, id: UniverseId) -> Option<String> {
        self.universes.read().get(&id.0).map(|u| u.label.clone())
    }

    pub fn residual(&self, id: UniverseId) -> Option<f64> {
        self.universes.read().get(&id.0).map(|u| u.residual)
    }

    /// Mutate a universe's manifold under exclusive lock.
    pub fn with_universe_mut<R>(
        &self,
        id: UniverseId,
        f: impl FnOnce(&mut UniverseBranch) -> R,
    ) -> LedgerResult<R> {
        let mut map = self.universes.write();
        let u = map
            .get_mut(&id.0)
            .ok_or(LedgerError::UnknownUniverse(id.0))?;
        Ok(f(u))
    }

    pub fn with_universe<R>(
        &self,
        id: UniverseId,
        f: impl FnOnce(&UniverseBranch) -> R,
    ) -> LedgerResult<R> {
        let map = self.universes.read();
        let u = map.get(&id.0).ok_or(LedgerError::UnknownUniverse(id.0))?;
        Ok(f(u))
    }

    pub fn live_ids(&self) -> Vec<UniverseId> {
        self.universes
            .read()
            .values()
            .filter(|u| u.is_live())
            .map(|u| u.id)
            .collect()
    }

    pub fn events(&self) -> Vec<BifurcationEvent> {
        self.events.read().clone()
    }

    /// Bifurcate from a MultiWeighted (or Unique) fixed-point solution.
    pub fn bifurcate_from_solution(
        &self,
        parent: UniverseId,
        solution: &FixedPointSolution,
        cause: ForkCause,
    ) -> LedgerResult<BifurcationEvent> {
        if *self.sealed.read() {
            return Err(LedgerError::LedgerSealed);
        }
        if !matches!(
            solution.class,
            ConvergenceClass::MultiWeighted | ConvergenceClass::Unique
        ) {
            return Err(LedgerError::InvalidWeights(0.0));
        }
        if solution.states.is_empty() {
            return Err(LedgerError::InvalidWeights(0.0));
        }

        let weights = if solution.weights.is_empty() {
            vec![1.0 / solution.states.len() as f64; solution.states.len()]
        } else {
            solution.weights.clone()
        };
        let wsum: f64 = weights.iter().sum();
        if wsum <= 0.0 {
            return Err(LedgerError::InvalidWeights(wsum));
        }
        let weights: Vec<f64> = weights.iter().map(|w| w / wsum).collect();

        let (parent_label, parent_gen, parent_snap) = {
            let map = self.universes.read();
            let p = map
                .get(&parent.0)
                .ok_or(LedgerError::UnknownUniverse(parent.0))?;
            if !p.is_live() {
                return Err(LedgerError::InactiveParent(parent.0));
            }
            (
                p.label.clone(),
                p.generation,
                clone_manifold(&p.dag),
            )
        };

        let mut child_ids = Vec::new();
        let mut out_weights = Vec::new();
        let mut out_states = Vec::new();

        for (i, (state, w)) in solution.states.iter().zip(weights.iter()).enumerate() {
            if *w < self.config.min_branch_weight {
                continue;
            }
            let id = self.alloc_uid();
            let mut dag = clone_manifold(&parent_snap);
            for (j, v) in state.iter().enumerate() {
                let addr = SpacetimeAddr::new(j as u64, 0);
                if dag.lookup(addr).is_none() {
                    dag.allocate(addr, NodeState::scalar(*v));
                } else {
                    let _ = dag.retro_write(addr, Arc::from([*v]));
                }
            }
            let branch = UniverseBranch {
                id,
                parent: Some(parent),
                status: UniverseStatus::Active,
                weight: *w,
                fixed_point: state.clone(),
                residual: solution.stats.final_residual,
                dag,
                generation: parent_gen + 1,
                label: format!("{}::fork[{i}]", parent_label),
            };
            self.universes.write().insert(id.0, branch);
            child_ids.push(id);
            out_weights.push(*w);
            out_states.push(state.clone());
        }

        if child_ids.is_empty() {
            return Err(LedgerError::InvalidWeights(0.0));
        }

        let bid = self.alloc_bid();
        let event = BifurcationEvent {
            id: bid,
            parent,
            children: child_ids,
            cause,
            weights: out_weights,
            fixed_points: out_states,
            residual: solution.stats.final_residual,
            note: format!(
                "bifurcation {:?} → {} child universes",
                cause,
                solution.states.len()
            ),
        };
        self.events.write().push(event.clone());
        Ok(event)
    }

    pub fn condemn(&self, id: UniverseId) -> LedgerResult<()> {
        let mut map = self.universes.write();
        let u = map
            .get_mut(&id.0)
            .ok_or(LedgerError::UnknownUniverse(id.0))?;
        if matches!(u.status, UniverseStatus::Collapsed | UniverseStatus::Pruned) {
            return Err(LedgerError::AlreadyTerminal(id.0));
        }
        u.status = UniverseStatus::Condemned;
        Ok(())
    }

    pub fn mark_collapsed(&self, id: UniverseId) -> LedgerResult<()> {
        let mut map = self.universes.write();
        let u = map
            .get_mut(&id.0)
            .ok_or(LedgerError::UnknownUniverse(id.0))?;
        u.status = UniverseStatus::Collapsed;
        Ok(())
    }

    pub fn mark_pruned(&self, id: UniverseId) -> LedgerResult<()> {
        let mut map = self.universes.write();
        let u = map
            .get_mut(&id.0)
            .ok_or(LedgerError::UnknownUniverse(id.0))?;
        u.status = UniverseStatus::Pruned;
        if !self.config.retain_collapsed_history {
            u.dag = WorldlineDag::new();
        }
        Ok(())
    }

    pub fn seal(&self) {
        *self.sealed.write() = true;
    }

    pub fn snapshot(&self) -> LedgerSnapshot {
        let branches: Vec<_> = self
            .universes
            .read()
            .values()
            .map(|u| (u.id.0, u.weight, u.status, u.label.clone()))
            .collect();
        LedgerSnapshot {
            primary: self.primary().0,
            live_universes: branches
                .iter()
                .filter(|(_, _, s, _)| matches!(s, UniverseStatus::Active))
                .count(),
            total_bifurcations: self.events.read().len(),
            sealed: *self.sealed.read(),
            branches,
        }
    }

    fn alloc_uid(&self) -> UniverseId {
        let mut n = self.next_uid.write();
        let id = UniverseId(*n);
        *n += 1;
        id
    }

    fn alloc_bid(&self) -> BifurcationId {
        let mut n = self.next_bid.write();
        let id = BifurcationId(*n);
        *n += 1;
        id
    }
}

fn clone_manifold(src: &WorldlineDag) -> WorldlineDag {
    let mut child = WorldlineDag::new();
    if let Ok(snap) = src.snapshot() {
        for (addr, val, weight) in snap.nodes {
            child.allocate(
                addr,
                NodeState {
                    value: Arc::clone(&val),
                    weight,
                    revision: 0,
                    pruned: false,
                },
            );
        }
    }
    child.branch_generation = src.branch_generation;
    child
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_kernel::SolverStats;

    #[test]
    fn bifurcates_multiweighted_into_child_universes() {
        let ledger = OmniversalLedger::default();
        let root = ledger.bootstrap("prime", WorldlineDag::new());
        let sol = FixedPointSolution {
            class: ConvergenceClass::MultiWeighted,
            states: vec![vec![0.0], vec![0.5], vec![1.0]],
            weights: vec![0.2, 0.5, 0.3],
            stats: SolverStats {
                iterations: 10,
                final_residual: 1e-12,
                restarts_used: 6,
                fixed_points_found: 3,
            },
        };
        let event = ledger
            .bifurcate_from_solution(root, &sol, ForkCause::MultiFixedPoint)
            .unwrap();
        assert_eq!(event.children.len(), 3);
        assert_eq!(ledger.live_ids().len(), 4); // root + 3 children
        let w: f64 = event.weights.iter().sum();
        assert!((w - 1.0).abs() < 1e-9);
    }
}
