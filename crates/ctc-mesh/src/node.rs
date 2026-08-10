use ctc_oracle::OracleEngine;
use ctc_signal::SignalDaemon;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Executes past-epoch frames; hosts oracle injection points.
    PastWorker,
    /// Executes future-epoch solvers; publishes temporal packets.
    FutureSolver,
    /// Relay / coordinator with no local chronal state.
    Relay,
}

/// A physical or simulated cluster participant.
pub struct MeshNode {
    pub id: NodeId,
    pub role: NodeRole,
    pub online: bool,
    pub signal: Arc<SignalDaemon>,
    pub oracle: OracleEngine,
}

impl MeshNode {
    pub fn new(id: impl Into<String>, role: NodeRole, signal: Arc<SignalDaemon>) -> Self {
        let oracle = OracleEngine::new(Arc::clone(&signal));
        Self {
            id: NodeId(id.into()),
            role,
            online: true,
            signal,
            oracle,
        }
    }

    pub fn set_online(&mut self, online: bool) {
        self.online = online;
    }
}
