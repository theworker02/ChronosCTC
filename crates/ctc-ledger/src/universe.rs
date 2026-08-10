use ctc_dag::{NodeState, SpacetimeAddr, WorldlineDag};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UniverseId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UniverseStatus {
    /// Actively evolving under its own manifold.
    Active,
    /// Marked suboptimal / paradoxical — pending prune.
    Condemned,
    /// Merged into the primary thread by the collapse engine.
    Collapsed,
    /// Purged; manifold released.
    Pruned,
}

/// An isolated reality branch with its own worldline manifold and weight.
pub struct UniverseBranch {
    pub id: UniverseId,
    pub parent: Option<UniverseId>,
    pub status: UniverseStatus,
    /// Deutsch mixture weight \(w_i\) inherited or assigned at bifurcation.
    pub weight: f64,
    /// Fixed-point state vector that founded this branch.
    pub fixed_point: Vec<f64>,
    pub residual: f64,
    /// Independent memory manifold for this universe.
    pub dag: WorldlineDag,
    pub generation: u64,
    pub label: String,
}

impl UniverseBranch {
    pub fn root(label: impl Into<String>, dag: WorldlineDag) -> Self {
        Self {
            id: UniverseId(0), // assigned by ledger
            parent: None,
            status: UniverseStatus::Active,
            weight: 1.0,
            fixed_point: Vec::new(),
            residual: 0.0,
            dag,
            generation: 0,
            label: label.into(),
        }
    }

    pub fn is_live(&self) -> bool {
        matches!(self.status, UniverseStatus::Active)
    }

    /// Seed manifold cells from a fixed-point vector (one scalar cell per coord).
    pub fn seed_from_fixed_point(&mut self, state: &[f64]) {
        self.fixed_point = state.to_vec();
        for (i, v) in state.iter().enumerate() {
            let addr = SpacetimeAddr::new(i as u64, 0);
            self.dag.allocate(addr, NodeState::scalar(*v));
        }
    }

    /// Clone manifold topology/values into a child branch skeleton.
    pub fn clone_manifold_snapshot(&self) -> WorldlineDag {
        let mut child = WorldlineDag::new();
        if let Ok(snap) = self.dag.snapshot() {
            for (addr, val, weight) in snap.nodes {
                let mut state = NodeState {
                    value: Arc::clone(&val),
                    weight,
                    revision: 0,
                    pruned: false,
                };
                // Keep weight on the cell.
                state.weight = weight;
                child.allocate(addr, state);
            }
        }
        child.branch_generation = self.dag.branch_generation;
        child
    }
}
