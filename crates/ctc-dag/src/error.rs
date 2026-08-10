use crate::addr::SpacetimeAddr;
use crate::node::NodeId;
use thiserror::Error;

pub type DagResult<T> = Result<T, DagError>;

#[derive(Debug, Error)]
pub enum DagError {
    #[error("spacetime address {0} is not allocated in the worldline fabric")]
    UnmappedAddress(SpacetimeAddr),

    #[error("node {0:?} is not present in the worldline DAG")]
    MissingNode(NodeId),

    #[error("introducing edge {from:?} → {to:?} would create a structural cycle outside a declared CTC region")]
    StructuralCycle { from: NodeId, to: NodeId },

    #[error("retro-write at {addr} rejected: target epoch is sealed (revision {revision})")]
    SealedEpoch { addr: SpacetimeAddr, revision: u64 },

    #[error("cascade depth exceeded limit {limit} while propagating from {origin}")]
    CascadeOverflow { origin: SpacetimeAddr, limit: usize },

    #[error("worldline snapshot is empty; no chronal state to commit")]
    EmptySnapshot,
}
