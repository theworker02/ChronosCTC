use crate::addr::SpacetimeAddr;
use crate::cascade::{CascadeReport, CascadeScheduler};
use crate::error::{DagError, DagResult};
use crate::node::{NodeId, NodeState, WorldlineNode};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Classification of a worldline edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyKind {
    /// Forward-in-time dataflow: \(\tau_i \rightarrow \tau_j\) with \(\tau_j \ge \tau_i\).
    Causal,
    /// Backward-in-time dataflow across a CTC: output at \(\tau_{t+1}\) feeds input at \(\tau_t\).
    Retrocausal,
    /// Intra-epoch algebraic coupling inside a nonlinear block.
    Algebraic,
}

#[derive(Clone, Debug)]
struct Edge {
    to: NodeId,
    kind: DependencyKind,
}

/// Immutable snapshot of chronal register values for kernel handoff.
#[derive(Clone, Debug)]
pub struct WorldlineSnapshot {
    pub nodes: Vec<(SpacetimeAddr, Arc<[f64]>, f64)>,
}

/// Memory-mapped immutable DAG of spacetime execution frames.
///
/// ## Indexing
///
/// Primary key: [`SpacetimeAddr`]. Secondary key: [`NodeId`].
/// Adjacency is stored as outgoing edges; dependents are derived for cascade.
pub struct WorldlineDag {
    next_id: u64,
    nodes: FxHashMap<NodeId, WorldlineNode>,
    by_addr: FxHashMap<SpacetimeAddr, NodeId>,
    edges: FxHashMap<NodeId, Vec<Edge>>,
    /// Reverse adjacency for O(1) cascade fan-out.
    reverse: FxHashMap<NodeId, Vec<NodeId>>,
    cascade: CascadeScheduler,
    /// Generation counter for branch identity (consumed by Paradox Pruner).
    pub branch_generation: u64,
    lock: RwLock<()>,
}

impl Default for WorldlineDag {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldlineDag {
    pub fn new() -> Self {
        Self {
            next_id: 1,
            nodes: FxHashMap::default(),
            by_addr: FxHashMap::default(),
            edges: FxHashMap::default(),
            reverse: FxHashMap::default(),
            cascade: CascadeScheduler::default(),
            branch_generation: 0,
            lock: RwLock::new(()),
        }
    }

    pub fn with_cascade_limit(mut self, max_depth: usize) -> Self {
        self.cascade = CascadeScheduler::new(max_depth);
        self
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, id: NodeId) -> Option<&WorldlineNode> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut WorldlineNode> {
        self.nodes.get_mut(&id)
    }

    pub fn lookup(&self, addr: SpacetimeAddr) -> Option<&WorldlineNode> {
        self.by_addr.get(&addr).and_then(|id| self.nodes.get(id))
    }

    pub fn node_id(&self, addr: SpacetimeAddr) -> Option<NodeId> {
        self.by_addr.get(&addr).copied()
    }

    /// Allocate a new spacetime cell. Re-allocating an existing address revises it
    /// only if unsealed; otherwise returns [`DagError::SealedEpoch`].
    pub fn allocate(&mut self, addr: SpacetimeAddr, state: NodeState) -> NodeId {
        let _guard = self.lock.write();
        if let Some(&existing) = self.by_addr.get(&addr) {
            if let Some(node) = self.nodes.get_mut(&existing) {
                if !node.sealed {
                    node.state = state;
                    node.dirty = true;
                    return existing;
                }
            }
        }
        let id = NodeId(self.next_id);
        self.next_id += 1;
        let node = WorldlineNode::new(id, addr, state);
        self.nodes.insert(id, node);
        self.by_addr.insert(addr, id);
        self.edges.entry(id).or_default();
        self.reverse.entry(id).or_default();
        id
    }

    /// Declare a dependency edge `from → to`.
    ///
    /// Retrocausal edges are explicitly permitted — they are the structural
    /// substrate of CTC regions. Structural cycles composed solely of
    /// `Causal` edges are rejected (those must be declared Retrocausal).
    pub fn add_dependency(
        &mut self,
        from: NodeId,
        to: NodeId,
        kind: DependencyKind,
    ) -> DagResult<()> {
        let _guard = self.lock.write();
        if !self.nodes.contains_key(&from) {
            return Err(DagError::MissingNode(from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(DagError::MissingNode(to));
        }

        if kind == DependencyKind::Causal && self.reaches(to, from) {
            // Purely causal path already connects to→from; adding from→to closes
            // a causal cycle without a CTC declaration.
            return Err(DagError::StructuralCycle { from, to });
        }

        self.edges.entry(from).or_default().push(Edge { to, kind });
        self.reverse.entry(to).or_default().push(from);
        Ok(())
    }

    /// Nodes that depend on `id` (consumers — cascade targets).
    pub fn dependents(&self, id: NodeId) -> Vec<NodeId> {
        self.edges
            .get(&id)
            .map(|es| es.iter().map(|e| e.to).collect())
            .unwrap_or_default()
    }

    /// Retroactive write: revise payload at `addr` and cascade dirty bits.
    ///
    /// A write at \(\tau_2\) that targets \(\tau_1 < \tau_2\) is the canonical
    /// retrocausal mutation. Dependents are invalidated for kernel re-solve.
    pub fn retro_write(
        &mut self,
        addr: SpacetimeAddr,
        value: Arc<[f64]>,
    ) -> DagResult<CascadeReport> {
        let id = self
            .by_addr
            .get(&addr)
            .copied()
            .ok_or(DagError::UnmappedAddress(addr))?;

        {
            let node = self.nodes.get_mut(&id).ok_or(DagError::MissingNode(id))?;
            if node.sealed {
                return Err(DagError::SealedEpoch {
                    addr,
                    revision: node.state.revision,
                });
            }
            *node = node.revise(value);
            node.dirty = true;
        }

        // Detach scheduler so propagate can take &mut self without split-borrow.
        let cascade = CascadeScheduler::new(self.cascade.max_depth);
        cascade.propagate(self, id)
    }

    /// Seal an epoch cell — freezes it against further revision.
    pub fn seal(&mut self, addr: SpacetimeAddr) -> DagResult<()> {
        let id = self
            .by_addr
            .get(&addr)
            .copied()
            .ok_or(DagError::UnmappedAddress(addr))?;
        let node = self.nodes.get_mut(&id).ok_or(DagError::MissingNode(id))?;
        node.sealed = true;
        Ok(())
    }

    /// Collect dirty nodes for the chronal kernel.
    pub fn dirty_nodes(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .filter(|(_, n)| n.dirty && !n.state.pruned)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Clear dirty flags after a successful fixed-point lock.
    pub fn clear_dirty(&mut self, ids: &[NodeId]) {
        for id in ids {
            if let Some(n) = self.nodes.get_mut(id) {
                n.dirty = false;
            }
        }
    }

    /// Mark nodes pruned (Paradox Pruner integration point).
    pub fn prune_nodes(&mut self, ids: &[NodeId]) {
        for id in ids {
            if let Some(n) = self.nodes.get_mut(id) {
                n.state.pruned = true;
                n.dirty = false;
            }
        }
        self.branch_generation = self.branch_generation.saturating_add(1);
    }

    /// Commit current non-pruned state into a kernel-consumable snapshot.
    pub fn snapshot(&self) -> DagResult<WorldlineSnapshot> {
        let mut nodes: Vec<_> = self
            .nodes
            .values()
            .filter(|n| !n.state.pruned)
            .map(|n| (n.addr, Arc::clone(&n.state.value), n.state.weight))
            .collect();
        if nodes.is_empty() {
            return Err(DagError::EmptySnapshot);
        }
        nodes.sort_by_key(|(addr, _, _)| *addr);
        Ok(WorldlineSnapshot { nodes })
    }

    /// Apply a converged state vector back onto dirty (or specified) nodes.
    ///
    /// `values` is parallel to the sorted snapshot order of `ids`.
    pub fn apply_solution(&mut self, ids: &[NodeId], values: &[Arc<[f64]>]) -> DagResult<()> {
        if ids.len() != values.len() {
            // Dimension mismatch is a programming error in the kernel bridge;
            // treat as missing node of a sentinel id for consistent error paths.
            return Err(DagError::MissingNode(NodeId(0)));
        }
        for (id, val) in ids.iter().zip(values.iter()) {
            let node = self.nodes.get_mut(id).ok_or(DagError::MissingNode(*id))?;
            *node = node.revise(Arc::clone(val));
            node.dirty = false;
        }
        Ok(())
    }

    /// Enumerate all retrocausal edges — CTC region surface for the compiler.
    pub fn retrocausal_edges(&self) -> Vec<(NodeId, NodeId)> {
        let mut out = Vec::new();
        for (from, edges) in &self.edges {
            for e in edges {
                if e.kind == DependencyKind::Retrocausal {
                    out.push((*from, e.to));
                }
            }
        }
        out
    }

    fn reaches(&self, from: NodeId, target: NodeId) -> bool {
        let mut stack = vec![from];
        let mut seen = FxHashSet::default();
        while let Some(n) = stack.pop() {
            if n == target {
                return true;
            }
            if !seen.insert(n) {
                continue;
            }
            if let Some(edges) = self.edges.get(&n) {
                for e in edges {
                    if e.kind == DependencyKind::Causal {
                        stack.push(e.to);
                    }
                }
            }
        }
        false
    }
}

// Import for reaches()
use rustc_hash::FxHashSet;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retro_write_cascades_and_revises() {
        let mut dag = WorldlineDag::new();
        let a0 = dag.allocate(SpacetimeAddr::new(1, 0), NodeState::scalar(0.0));
        let a1 = dag.allocate(SpacetimeAddr::new(1, 1), NodeState::scalar(1.0));
        dag.add_dependency(a1, a0, DependencyKind::Retrocausal)
            .unwrap();

        let report = dag.retro_write(SpacetimeAddr::new(1, 1), Arc::from([0.5])).unwrap();
        assert!(report.invalidated.contains(&a1));
        assert!(report.invalidated.contains(&a0));
        assert!((dag.node(a1).unwrap().state.value[0] - 0.5).abs() < 1e-12);
        assert_eq!(dag.node(a1).unwrap().state.revision, 1);
    }

    #[test]
    fn causal_cycle_without_ctc_is_rejected() {
        let mut dag = WorldlineDag::new();
        let a = dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.0));
        let b = dag.allocate(SpacetimeAddr::new(0, 1), NodeState::scalar(0.0));
        dag.add_dependency(a, b, DependencyKind::Causal).unwrap();
        let err = dag.add_dependency(b, a, DependencyKind::Causal).unwrap_err();
        assert!(matches!(err, DagError::StructuralCycle { .. }));
    }
}
