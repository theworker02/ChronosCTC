use crate::addr::SpacetimeAddr;
use crate::error::{DagError, DagResult};
use crate::node::NodeId;
use crate::worldline::WorldlineDag;
use rustc_hash::FxHashSet;

/// Report emitted after a retro-write cascade completes.
#[derive(Clone, Debug, Default)]
pub struct CascadeReport {
    pub origin: Option<SpacetimeAddr>,
    pub invalidated: Vec<NodeId>,
    pub depth_reached: usize,
}

/// Schedules dirty-bit propagation across worldline dependency edges.
///
/// ## Algorithm
///
/// Breadth-first traversal from the origin node along outgoing dependency
/// edges. Each visited node is marked `dirty = true`. Depth is bounded to
/// prevent runaway paradox feedback before the Paradox Pruner intervenes.
pub struct CascadeScheduler {
    pub max_depth: usize,
}

impl Default for CascadeScheduler {
    fn default() -> Self {
        Self { max_depth: 10_000 }
    }
}

impl CascadeScheduler {
    pub fn new(max_depth: usize) -> Self {
        Self { max_depth }
    }

    /// Invalidate `origin` and all nodes in its forward dependency closure.
    pub fn propagate(&self, dag: &mut WorldlineDag, origin: NodeId) -> DagResult<CascadeReport> {
        let origin_addr = dag
            .node(origin)
            .ok_or(DagError::MissingNode(origin))?
            .addr;

        let mut report = CascadeReport {
            origin: Some(origin_addr),
            invalidated: Vec::new(),
            depth_reached: 0,
        };

        let mut visited: FxHashSet<NodeId> = FxHashSet::default();
        let mut frontier: Vec<(NodeId, usize)> = vec![(origin, 0)];

        while let Some((nid, depth)) = frontier.pop() {
            if depth > self.max_depth {
                return Err(DagError::CascadeOverflow {
                    origin: origin_addr,
                    limit: self.max_depth,
                });
            }
            if !visited.insert(nid) {
                continue;
            }

            {
                let node = dag.node_mut(nid).ok_or(DagError::MissingNode(nid))?;
                node.dirty = true;
            }
            report.invalidated.push(nid);
            report.depth_reached = report.depth_reached.max(depth);

            let dependents = dag.dependents(nid);
            for dep in dependents {
                if !visited.contains(&dep) {
                    frontier.push((dep, depth + 1));
                }
            }
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeState;
    use crate::worldline::DependencyKind;

    #[test]
    fn cascade_marks_transitive_dependents_dirty() {
        let mut dag = WorldlineDag::new();
        let a = dag.allocate(SpacetimeAddr::new(0, 0), NodeState::scalar(0.0));
        let b = dag.allocate(SpacetimeAddr::new(0, 1), NodeState::scalar(0.0));
        let c = dag.allocate(SpacetimeAddr::new(0, 2), NodeState::scalar(0.0));
        dag.add_dependency(a, b, DependencyKind::Causal).unwrap();
        dag.add_dependency(b, c, DependencyKind::Retrocausal)
            .unwrap();

        let report = CascadeScheduler::default().propagate(&mut dag, a).unwrap();
        assert_eq!(report.invalidated.len(), 3);
        assert!(dag.node(c).unwrap().dirty);
    }
}
