//! # Worldline Memory Fabric (`ctc-dag`)
//!
//! Memory is not a flat address space. Every cell is a point on a topological
//! spacetime manifold indexed by \((a, \tau)\), where \(a\) is a logical address
//! and \(\tau\) is the proper-time / epoch coordinate.
//!
//! ## Immutability contract
//!
//! Nodes are append-only. A "write" at \(\tau_2\) that retrocausally affects
//! \(\tau_1\) does **not** mutate the historical node in place; it allocates a
//! new epoch revision and marks dependent worldline edges dirty, triggering a
//! cascade recompute across the DAG.
//!
//! ## Cascade semantics
//!
//! Let \(G = (V, E)\) be the worldline DAG. A retro-write at node \(v\)
//! invalidates the transitive closure of nodes reachable from \(v\) along
//! causal (and retrocausal) dependency edges, then schedules them for
//! fixed-point re-resolution by `ctc-kernel`.

mod addr;
mod cascade;
mod error;
mod node;
mod worldline;

pub use addr::{Epoch, LogicalAddr, SpacetimeAddr};
pub use cascade::{CascadeReport, CascadeScheduler};
pub use error::{DagError, DagResult};
pub use node::{NodeId, NodeState, WorldlineNode};
pub use worldline::{DependencyKind, WorldlineDag, WorldlineSnapshot};
