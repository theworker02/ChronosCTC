use crate::bind::{worldline_fingerprint, WorldlineBinding};
use crate::config::SignalConfig;
use crate::error::{SignalError, SignalResult};
use crate::inject::{InjectionReceipt, MemoryInjector};
use crate::packet::{
    ExpectedFootprint, PacketKind, PayloadCell, TemporalPacket, TemporalPacketBuilder,
};
use ctc_dag::{Epoch, SpacetimeAddr, WorldlineDag};
use ctc_kernel::NonlinearSystem;
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransmitReport {
    pub packet_id: u64,
    pub injection: InjectionReceipt,
    pub source_tau: i64,
    pub target_tau: i64,
}

/// Retrocausal signal daemon — packages future state and injects into the past.
pub struct SignalDaemon {
    pub config: SignalConfig,
    builder: RwLock<TemporalPacketBuilder>,
    injector: MemoryInjector,
    /// Registered past footprints awaiting packets (keyed by footprint hash).
    footprints: RwLock<FxHashMap<u64, ExpectedFootprint>>,
    /// In-flight packets awaiting delivery (mailbox).
    mailbox: RwLock<FxHashMap<u64, TemporalPacket>>,
}

impl Default for SignalDaemon {
    fn default() -> Self {
        Self::new(SignalConfig::default())
    }
}

impl SignalDaemon {
    pub fn new(config: SignalConfig) -> Self {
        Self {
            injector: MemoryInjector::new(config.clone()),
            config,
            builder: RwLock::new(TemporalPacketBuilder::new()),
            footprints: RwLock::new(FxHashMap::default()),
            mailbox: RwLock::new(FxHashMap::default()),
        }
    }

    /// Register a past-epoch injection footprint (call at \(\tau_0\) before wait).
    pub fn register_footprint(&self, fp: ExpectedFootprint) -> u64 {
        let hash = fp.hash;
        self.footprints.write().insert(hash, fp);
        hash
    }

    pub fn footprint(&self, hash: u64) -> Option<ExpectedFootprint> {
        self.footprints.read().get(&hash).cloned()
    }

    /// Capture binding from the current DAG for footprint registration.
    pub fn bind(&self, dag: &WorldlineDag) -> WorldlineBinding {
        worldline_fingerprint(dag)
    }

    /// Package a future-resolved payload into an immutable temporal packet.
    pub fn package(
        &self,
        kind: PacketKind,
        source_tau: Epoch,
        target_tau: Epoch,
        binding: WorldlineBinding,
        cells: Vec<PayloadCell>,
    ) -> SignalResult<TemporalPacket> {
        if cells.len() > self.config.max_payload_cells {
            return Err(SignalError::FootprintMismatch {
                expected: self.config.max_payload_cells,
                got: cells.len(),
            });
        }
        self.builder
            .write()
            .package(kind, source_tau, target_tau, binding, cells)
    }

    pub fn package_scalars(
        &self,
        source_tau: Epoch,
        target_tau: Epoch,
        binding: WorldlineBinding,
        pairs: &[(SpacetimeAddr, f64)],
    ) -> SignalResult<TemporalPacket> {
        self.builder
            .write()
            .package_scalars(source_tau, target_tau, binding, pairs)
    }

    /// Stage a packet in the mailbox for oracle pickup / mesh relay.
    pub fn stage(&self, packet: TemporalPacket) -> u64 {
        let id = packet.id;
        self.mailbox.write().insert(id, packet);
        id
    }

    pub fn take(&self, packet_id: u64) -> Option<TemporalPacket> {
        self.mailbox.write().remove(&packet_id)
    }

    pub fn peek_for_target(&self, target_tau: Epoch) -> Option<TemporalPacket> {
        self.mailbox
            .read()
            .values()
            .find(|p| p.target_tau == target_tau)
            .cloned()
    }

    /// Full lifecycle: validate + inject immediately against a registered footprint.
    pub fn transmit(
        &self,
        dag: &mut WorldlineDag,
        packet: &TemporalPacket,
        system: Option<&NonlinearSystem>,
    ) -> SignalResult<TransmitReport> {
        let expected = self
            .footprints
            .read()
            .get(&packet.footprint_hash)
            .cloned()
            .ok_or(SignalError::FootprintMismatch {
                expected: 0,
                got: packet.cells.len(),
            })?;

        let injection = self.injector.inject(dag, packet, &expected, system)?;
        Ok(TransmitReport {
            packet_id: packet.id,
            injection,
            source_tau: packet.source_tau.0,
            target_tau: packet.target_tau.0,
        })
    }

    /// Deliver a staged packet to its registered past footprint.
    pub fn deliver_staged(
        &self,
        dag: &mut WorldlineDag,
        packet_id: u64,
        system: Option<&NonlinearSystem>,
    ) -> SignalResult<TransmitReport> {
        let packet = self.take(packet_id).ok_or(SignalError::EmptyPayload)?;
        self.transmit(dag, &packet, system)
    }
}

/// Shared handle for mesh / oracle integration.
pub type SharedSignalDaemon = Arc<SignalDaemon>;

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_dag::{NodeState, SpacetimeAddr};
    use std::sync::Arc;

    #[test]
    fn teleport_future_state_into_past_epoch() {
        let daemon = SignalDaemon::default();
        let mut dag = WorldlineDag::new();
        let past = SpacetimeAddr::new(0, 0);
        let future = SpacetimeAddr::new(0, 5);
        dag.allocate(past, NodeState::scalar(0.0));
        dag.allocate(future, NodeState::scalar(0.0));

        let binding = daemon.bind(&dag);
        let cells = vec![PayloadCell {
            addr: past,
            values: vec![0.42],
            blob: vec![],
        }];
        let fp = ExpectedFootprint::from_cells(Epoch(0), &cells, binding);
        daemon.register_footprint(fp);

        // Future converges...
        dag.retro_write(future, Arc::from([0.42])).unwrap();

        let packet = daemon
            .package_scalars(Epoch(5), Epoch(0), binding, &[(past, 0.42)])
            .unwrap();
        let report = daemon.transmit(&mut dag, &packet, None).unwrap();
        assert_eq!(report.injection.cells_written, 1);
        assert!((dag.lookup(past).unwrap().state.value[0] - 0.42).abs() < 1e-12);
    }

    #[test]
    fn rejects_non_retrocausal_packet() {
        let daemon = SignalDaemon::default();
        let err = daemon
            .package_scalars(
                Epoch(1),
                Epoch(2),
                WorldlineBinding {
                    fingerprint: 0,
                    generation: 0,
                },
                &[(SpacetimeAddr::new(0, 2), 1.0)],
            )
            .unwrap_err();
        assert!(matches!(err, SignalError::NonRetrocausal { .. }));
    }
}
