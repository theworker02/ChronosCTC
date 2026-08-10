use crate::universe::UniverseId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BifurcationId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForkCause {
    /// Multiple Deutsch fixed points from the chronal kernel.
    MultiFixedPoint,
    /// Agent-requested speculative exploration fork.
    AgentProbe,
    /// Mesh / distributed ambiguity.
    DistributedAmbiguity,
}

/// Append-only chronal bifurcation event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BifurcationEvent {
    pub id: BifurcationId,
    pub parent: UniverseId,
    pub children: Vec<UniverseId>,
    pub cause: ForkCause,
    pub weights: Vec<f64>,
    pub fixed_points: Vec<Vec<f64>>,
    pub residual: f64,
    pub note: String,
}
