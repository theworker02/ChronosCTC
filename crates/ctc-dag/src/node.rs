use crate::addr::SpacetimeAddr;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Stable identity of a worldline node within a [`crate::WorldlineDag`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

/// Payload carried by a spacetime cell.
///
/// Values are dense `f64` vectors so the chronal kernel can treat classical
/// bits, continuous CTC registers, and (flattened) density-matrix diagonals
/// uniformly under fixed-point iteration.
///
/// `value` uses `Arc<[f64]>` for structural sharing across immutable revisions;
/// serde is implemented via a `Vec<f64>` projection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeState {
    /// Chronal register payload \(x \in \mathbb{R}^n\).
    #[serde(with = "arc_f64_slice")]
    pub value: Arc<[f64]>,
    /// Optional probability mass when this node is one of several fixed points.
    pub weight: f64,
    /// Monotonic revision counter; incremented on every retro-write cascade.
    pub revision: u64,
    /// When true, the Paradox Pruner has invalidated this node.
    pub pruned: bool,
}

mod arc_f64_slice {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::sync::Arc;

    pub fn serialize<S: Serializer>(v: &Arc<[f64]>, s: S) -> Result<S::Ok, S::Error> {
        v.as_ref().serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Arc<[f64]>, D::Error> {
        let v = Vec::<f64>::deserialize(d)?;
        Ok(Arc::from(v.into_boxed_slice()))
    }
}

impl NodeState {
    pub fn scalar(x: f64) -> Self {
        Self {
            value: Arc::from([x]),
            weight: 1.0,
            revision: 0,
            pruned: false,
        }
    }

    pub fn from_slice(xs: &[f64]) -> Self {
        Self {
            value: Arc::from(xs),
            weight: 1.0,
            revision: 0,
            pruned: false,
        }
    }

    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }

    /// L2 distance between payloads — used as a local residual contribution.
    pub fn l2_distance(&self, other: &Self) -> f64 {
        let n = self.value.len().min(other.value.len());
        let mut acc = 0.0;
        for i in 0..n {
            let d = self.value[i] - other.value[i];
            acc += d * d;
        }
        let dim_penalty = (self.value.len() as isize - other.value.len() as isize).unsigned_abs();
        acc += dim_penalty as f64;
        acc.sqrt()
    }
}

/// Immutable worldline node at spacetime coordinate \((a, \tau)\).
#[derive(Clone, Debug)]
pub struct WorldlineNode {
    pub id: NodeId,
    pub addr: SpacetimeAddr,
    pub state: NodeState,
    /// When sealed, further in-place revision is forbidden; only successor
    /// epochs may be allocated.
    pub sealed: bool,
    /// Dirty flag set by cascade propagation pending kernel re-solve.
    pub dirty: bool,
}

impl WorldlineNode {
    pub fn new(id: NodeId, addr: SpacetimeAddr, state: NodeState) -> Self {
        Self {
            id,
            addr,
            state,
            sealed: false,
            dirty: false,
        }
    }

    /// Produce a successor revision with updated payload (immutability-preserving).
    pub fn revise(&self, value: Arc<[f64]>) -> Self {
        let mut next = self.clone();
        next.state.value = value;
        next.state.revision = self.state.revision.saturating_add(1);
        next.dirty = false;
        next
    }
}
