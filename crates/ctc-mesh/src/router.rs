use crate::channel::{EntanglementChannel, EntanglementId, MeshEnvelope};
use crate::config::MeshConfig;
use crate::error::{MeshError, MeshResult};
use crate::node::{MeshNode, NodeId, NodeRole};
use ctc_dag::WorldlineDag;
use ctc_oracle::SuperpositionState;
use ctc_signal::{SignalDaemon, TemporalPacket};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryReport {
    pub entanglement: u64,
    pub from: String,
    pub to: String,
    pub injection_point: String,
    pub packet_id: u64,
    pub cycles_saved: u64,
    pub hops: u32,
    pub message: String,
}

/// Cluster-wide registry of mesh nodes and entanglement channels.
pub struct MeshCluster {
    pub config: MeshConfig,
    nodes: RwLock<FxHashMap<String, Arc<RwLock<MeshNode>>>>,
    channels: RwLock<FxHashMap<u64, Arc<EntanglementChannel>>>,
    next_entanglement: RwLock<u64>,
}

impl Default for MeshCluster {
    fn default() -> Self {
        Self::new(MeshConfig::default())
    }
}

impl MeshCluster {
    pub fn new(config: MeshConfig) -> Self {
        Self {
            config,
            nodes: RwLock::new(FxHashMap::default()),
            channels: RwLock::new(FxHashMap::default()),
            next_entanglement: RwLock::new(1),
        }
    }

    pub fn register_node(&self, node: MeshNode) -> MeshResult<Arc<RwLock<MeshNode>>> {
        let mut nodes = self.nodes.write();
        if nodes.contains_key(&node.id.0) {
            return Err(MeshError::DuplicateNode(node.id.0));
        }
        let id = node.id.0.clone();
        let handle = Arc::new(RwLock::new(node));
        nodes.insert(id, Arc::clone(&handle));
        Ok(handle)
    }

    pub fn node(&self, id: &str) -> MeshResult<Arc<RwLock<MeshNode>>> {
        self.nodes
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| MeshError::UnknownNode(id.into()))
    }

    /// Create a temporal entanglement between two nodes.
    pub fn entangle(&self, a: &str, b: &str) -> MeshResult<EntanglementId> {
        let _ = self.node(a)?;
        let _ = self.node(b)?;
        let mut next = self.next_entanglement.write();
        let id = EntanglementId(*next);
        *next += 1;
        let ch = Arc::new(EntanglementChannel::new(
            id,
            NodeId(a.into()),
            NodeId(b.into()),
            self.config.channel_capacity,
        ));
        self.channels.write().insert(id.0, ch);
        Ok(id)
    }

    pub fn channel(&self, id: EntanglementId) -> MeshResult<Arc<EntanglementChannel>> {
        self.channels
            .read()
            .get(&id.0)
            .cloned()
            .ok_or(MeshError::UnknownChannel(id.0))
    }
}

/// Routes temporal packets across entangled nodes.
pub struct MeshRouter {
    pub cluster: Arc<MeshCluster>,
}

impl MeshRouter {
    pub fn new(cluster: Arc<MeshCluster>) -> Self {
        Self { cluster }
    }

    /// Publish a future-resolved packet from `from` toward `to`'s injection point.
    pub fn publish(
        &self,
        entanglement: EntanglementId,
        from: &str,
        to: &str,
        injection_point: impl Into<String>,
        packet: TemporalPacket,
    ) -> MeshResult<()> {
        let from_node = self.cluster.node(from)?;
        let to_node = self.cluster.node(to)?;
        if !from_node.read().online {
            return Err(MeshError::NodeOffline(from.into()));
        }
        if !to_node.read().online {
            return Err(MeshError::NodeOffline(to.into()));
        }

        let ch = self.cluster.channel(entanglement)?;
        if !ch.connects(&NodeId(from.into()), &NodeId(to.into())) {
            return Err(MeshError::NoRoute {
                from: from.into(),
                to: to.into(),
                entanglement: entanglement.0,
            });
        }

        // Cost-model hop spin.
        let spins = self.cluster.config.hop_latency_us;
        let mut acc = 0u64;
        for i in 0..spins {
            acc = acc.wrapping_add(i);
        }
        std::hint::black_box(acc);

        let env = MeshEnvelope {
            entanglement,
            from: NodeId(from.into()),
            to: NodeId(to.into()),
            injection_point: injection_point.into(),
            packet,
            hop_count: 1,
        };
        if !ch.push(env) {
            return Err(MeshError::Signal("channel capacity exceeded".into()));
        }
        Ok(())
    }

    /// Drain one envelope destined for `to` and collapse via the local oracle.
    pub fn deliver(
        &self,
        entanglement: EntanglementId,
        to: &str,
        dag: &mut WorldlineDag,
    ) -> MeshResult<DeliveryReport> {
        let ch = self.cluster.channel(entanglement)?;
        let env = ch
            .pop_for(&NodeId(to.into()))
            .ok_or(MeshError::Signal("no pending envelope".into()))?;

        let node = self.cluster.node(to)?;
        let node = node.write();
        if !node.online {
            return Err(MeshError::NodeOffline(to.into()));
        }

        // Ensure oracle is awaiting if configured.
        if self.cluster.config.require_awaiting {
            match node.oracle.status(&env.injection_point) {
                Some(SuperpositionState::Awaiting) => {}
                Some(other) => {
                    return Err(MeshError::Oracle(format!(
                        "injection point state {:?} ≠ Awaiting",
                        other
                    )));
                }
                None => {
                    return Err(MeshError::Oracle(format!(
                        "unknown injection point {}",
                        env.injection_point
                    )));
                }
            }
        }

        let report = node
            .oracle
            .collapse_with_packet(dag, &env.injection_point, env.packet.clone(), env.hop_count as u64)
            .map_err(|e| MeshError::Oracle(e.to_string()))?;

        let from = env.from.0;
        let to = env.to.0;
        Ok(DeliveryReport {
            entanglement: entanglement.0,
            injection_point: env.injection_point,
            packet_id: env.packet.id,
            cycles_saved: report.cycles_saved,
            hops: env.hop_count,
            message: format!(
                "mesh delivery {}→{} via E{}: {}",
                from, to, entanglement.0, report.message
            ),
            from,
            to,
        })
    }
}

/// Bootstrap a minimal two-node past/future pair sharing signal daemons.
pub fn bootstrap_pair(
    past_id: &str,
    future_id: &str,
) -> MeshResult<(Arc<MeshCluster>, EntanglementId, Arc<RwLock<MeshNode>>, Arc<RwLock<MeshNode>>)>
{
    let cluster = Arc::new(MeshCluster::default());
    // Each node has its own signal daemon — packets cross via mesh envelopes.
    let past_signal = Arc::new(SignalDaemon::default());
    let future_signal = Arc::new(SignalDaemon::default());

    let past = cluster.register_node(MeshNode::new(
        past_id,
        NodeRole::PastWorker,
        past_signal,
    ))?;
    let future = cluster.register_node(MeshNode::new(
        future_id,
        NodeRole::FutureSolver,
        future_signal,
    ))?;
    let eid = cluster.entangle(past_id, future_id)?;
    Ok((cluster, eid, past, future))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_dag::{Epoch, LogicalAddr, NodeState, SpacetimeAddr};
    use ctc_oracle::InjectionPoint;

    #[test]
    fn distributed_future_dictates_past_path() {
        let (cluster, eid, past_h, future_h) =
            bootstrap_pair("worker-past", "solver-future").unwrap();
        let router = MeshRouter::new(Arc::clone(&cluster));

        let mut past_dag = WorldlineDag::new();
        let addr = LogicalAddr(1);
        past_dag.allocate(SpacetimeAddr::new(1, 0), NodeState::scalar(0.0));

        {
            let past = past_h.write();
            past.oracle
                .register(
                    &past_dag,
                    InjectionPoint {
                        name: "ml_hparams".into(),
                        epoch: Epoch(0),
                        slots: vec![(addr, 1)],
                        estimated_cycles: 1_000_000,
                    },
                )
                .unwrap();
            past.oracle.enter_await("ml_hparams").unwrap();
        }

        // Future publishes a packet that matches the past footprint/binding.
        let packet = {
            let past = past_h.write();
            let id = past
                .oracle
                .stage_future_scalars("ml_hparams", Epoch(9), &[(addr, 0.618)])
                .unwrap();
            past.signal.take(id).unwrap()
        };

        router
            .publish(eid, "solver-future", "worker-past", "ml_hparams", packet)
            .unwrap();

        let delivery = router.deliver(eid, "worker-past", &mut past_dag).unwrap();
        assert_eq!(delivery.cycles_saved, 1_000_000);
        assert!(
            (past_dag.lookup(SpacetimeAddr::new(1, 0)).unwrap().state.value[0] - 0.618).abs()
                < 1e-12
        );

        let _ = future_h;
    }
}
