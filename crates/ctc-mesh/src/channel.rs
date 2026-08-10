use crate::node::NodeId;
use ctc_signal::TemporalPacket;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntanglementId(pub u64);

/// Envelope wrapping a temporal packet for inter-node transport.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshEnvelope {
    pub entanglement: EntanglementId,
    pub from: NodeId,
    pub to: NodeId,
    /// Oracle injection point name on the destination node.
    pub injection_point: String,
    pub packet: TemporalPacket,
    pub hop_count: u32,
}

/// Bidirectional mailbox for a logical entanglement between nodes.
pub struct EntanglementChannel {
    pub id: EntanglementId,
    pub endpoints: (NodeId, NodeId),
    capacity: usize,
    queue: Mutex<VecDeque<MeshEnvelope>>,
}

impl EntanglementChannel {
    pub fn new(id: EntanglementId, a: NodeId, b: NodeId, capacity: usize) -> Self {
        Self {
            id,
            endpoints: (a, b),
            capacity: capacity.max(1),
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn connects(&self, a: &NodeId, b: &NodeId) -> bool {
        (&self.endpoints.0 == a && &self.endpoints.1 == b)
            || (&self.endpoints.0 == b && &self.endpoints.1 == a)
    }

    pub fn push(&self, env: MeshEnvelope) -> bool {
        let mut q = self.queue.lock();
        if q.len() >= self.capacity {
            return false;
        }
        q.push_back(env);
        true
    }

    pub fn pop_for(&self, to: &NodeId) -> Option<MeshEnvelope> {
        let mut q = self.queue.lock();
        let idx = q.iter().position(|e| &e.to == to)?;
        q.remove(idx)
    }

    pub fn len(&self) -> usize {
        self.queue.lock().len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.lock().is_empty()
    }
}
