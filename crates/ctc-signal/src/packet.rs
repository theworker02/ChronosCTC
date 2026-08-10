use crate::bind::WorldlineBinding;
use crate::error::{SignalError, SignalResult};
use ctc_dag::{Epoch, LogicalAddr, SpacetimeAddr};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Classification of teleportation payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PacketKind {
    /// State differential \(\Delta x\) for chronal registers.
    StateDifferential,
    /// Binary hot-patch blob (opaque bytes interpreted by oracle hooks).
    HotPatch,
    /// Solver / execution parameters (flat f64 vector).
    ExecParams,
}

/// One cell of a temporal packet — maps onto a past spacetime address.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PayloadCell {
    pub addr: SpacetimeAddr,
    pub values: Vec<f64>,
    /// Optional opaque hot-patch bytes (PacketKind::HotPatch).
    pub blob: Vec<u8>,
}

/// Immutable retrocausal transport packet.
///
/// Bound to worldline hash \(H(W)\) at packaging time. Travel direction is
/// always \(\tau_{\mathrm{source}} > \tau_{\mathrm{target}}\).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemporalPacket {
    pub id: u64,
    pub kind: PacketKind,
    pub source_tau: Epoch,
    pub target_tau: Epoch,
    pub binding: WorldlineBinding,
    pub cells: Vec<PayloadCell>,
    /// Expected footprint signature (hash of target addresses + dims).
    pub footprint_hash: u64,
}

impl TemporalPacket {
    pub fn footprint_len(&self) -> usize {
        self.cells.len()
    }

    pub fn target_addresses(&self) -> Vec<SpacetimeAddr> {
        self.cells.iter().map(|c| c.addr).collect()
    }
}

/// Builder for packaging a future-resolved state into a temporal packet.
pub struct TemporalPacketBuilder {
    next_id: u64,
}

impl Default for TemporalPacketBuilder {
    fn default() -> Self {
        Self { next_id: 1 }
    }
}

impl TemporalPacketBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn package(
        &mut self,
        kind: PacketKind,
        source_tau: Epoch,
        target_tau: Epoch,
        binding: WorldlineBinding,
        cells: Vec<PayloadCell>,
    ) -> SignalResult<TemporalPacket> {
        if cells.is_empty() {
            return Err(SignalError::EmptyPayload);
        }
        if source_tau.0 <= target_tau.0 {
            return Err(SignalError::NonRetrocausal {
                from_tau: source_tau.0,
                to_tau: target_tau.0,
            });
        }
        let footprint_hash = footprint_hash(&cells);
        let id = self.next_id;
        self.next_id += 1;
        Ok(TemporalPacket {
            id,
            kind,
            source_tau,
            target_tau,
            binding,
            cells,
            footprint_hash,
        })
    }

    /// Convenience: package scalar differentials from `(addr, value)` pairs.
    pub fn package_scalars(
        &mut self,
        source_tau: Epoch,
        target_tau: Epoch,
        binding: WorldlineBinding,
        pairs: &[(SpacetimeAddr, f64)],
    ) -> SignalResult<TemporalPacket> {
        let cells = pairs
            .iter()
            .map(|(addr, v)| PayloadCell {
                addr: *addr,
                values: vec![*v],
                blob: Vec::new(),
            })
            .collect();
        self.package(
            PacketKind::StateDifferential,
            source_tau,
            target_tau,
            binding,
            cells,
        )
    }
}

/// Hash of the expected variable footprint (addresses + dimensions).
pub fn footprint_hash(cells: &[PayloadCell]) -> u64 {
    use rustc_hash::FxHasher;
    use std::hash::{Hash, Hasher};
    let mut h = FxHasher::default();
    for c in cells {
        c.addr.address.0.hash(&mut h);
        c.addr.tau.0.hash(&mut h);
        c.values.len().hash(&mut h);
        c.blob.len().hash(&mut h);
    }
    h.finish()
}

/// Expected footprint registered by a past epoch before it completes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExpectedFootprint {
    pub target_tau: Epoch,
    pub slots: Vec<(LogicalAddr, usize)>,
    pub hash: u64,
    /// Worldline topology binding captured at registration (pre-injection).
    pub binding: WorldlineBinding,
}

impl ExpectedFootprint {
    pub fn new(
        target_tau: Epoch,
        slots: Vec<(LogicalAddr, usize)>,
        binding: WorldlineBinding,
    ) -> Self {
        use rustc_hash::FxHasher;
        use std::hash::{Hash, Hasher};
        let mut h = FxHasher::default();
        for (a, dim) in &slots {
            a.0.hash(&mut h);
            target_tau.0.hash(&mut h);
            dim.hash(&mut h);
        }
        Self {
            target_tau,
            hash: h.finish(),
            slots,
            binding,
        }
    }

    /// Build footprint from concrete payload cells (same layout the future must send).
    pub fn from_cells(
        target_tau: Epoch,
        cells: &[PayloadCell],
        binding: WorldlineBinding,
    ) -> Self {
        let slots = cells
            .iter()
            .map(|c| {
                (
                    c.addr.address,
                    if c.values.is_empty() {
                        c.blob.len().max(1)
                    } else {
                        c.values.len()
                    },
                )
            })
            .collect();
        // Hash must match packet.footprint_hash (== footprint_hash(cells)).
        Self {
            target_tau,
            slots,
            hash: footprint_hash(cells),
            binding,
        }
    }
}

/// Helper to build a payload cell from an Arc slice.
pub fn cell_from_arc(addr: SpacetimeAddr, values: Arc<[f64]>) -> PayloadCell {
    PayloadCell {
        addr,
        values: values.iter().copied().collect(),
        blob: Vec::new(),
    }
}
