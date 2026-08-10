//! # Distributed Temporal Entanglement (`ctc-mesh`)
//!
//! Scales retrocausal signaling across independent physical nodes. A worker
//! running a heavy optimization at \(\tau_0\) can receive its final convergence
//! parameters from a future execution thread on a separate machine, forming a
//! clustered network where future outcomes dictate past processing paths.
//!
//! ## Entanglement channel
//!
//! Nodes share a logical entanglement id \(E\). A packet published on node \(A\)
//! at \(\tau_{n+1}\) is routed to the oracle mailbox on node \(B\) targeting
//! \(\tau_0\), preserving Deutsch-gate validation at the sink.

mod channel;
mod config;
mod error;
mod node;
mod router;

pub use channel::{EntanglementChannel, EntanglementId, MeshEnvelope};
pub use config::MeshConfig;
pub use error::{MeshError, MeshResult};
pub use node::{MeshNode, NodeId, NodeRole};
pub use router::{bootstrap_pair, DeliveryReport, MeshCluster, MeshRouter};
