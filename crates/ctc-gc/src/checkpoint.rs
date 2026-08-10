use crate::error::{GcError, GcResult};
use ctc_dag::{Epoch, SpacetimeAddr};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Immutable compressed snapshot of a sealed historical epoch slice.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EpochCheckpoint {
    pub id: u64,
    pub tau_lo: i64,
    pub tau_hi: i64,
    /// Packed cells: (logical address, τ, values).
    pub cells: Vec<(u64, i64, Vec<f64>)>,
    pub bytes_raw: usize,
    pub bytes_compressed: usize,
}

/// In-memory checkpoint archive — Phase-2 stand-in for mmap'd cold storage.
pub struct CheckpointStore {
    next_id: RwLock<u64>,
    capacity: usize,
    entries: RwLock<FxHashMap<u64, Arc<EpochCheckpoint>>>,
}

impl CheckpointStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            next_id: RwLock::new(1),
            capacity,
            entries: RwLock::new(FxHashMap::default()),
        }
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    pub fn total_compressed_bytes(&self) -> usize {
        self.entries
            .read()
            .values()
            .map(|c| c.bytes_compressed)
            .sum()
    }

    /// Compress a set of sealed cells into a checkpoint.
    ///
    /// Compression model: drop weight/revision metadata and delta-encode
    /// successive scalar values along each worldline (run-length friendly).
    pub fn compress(
        &self,
        cells: Vec<(SpacetimeAddr, Arc<[f64]>)>,
    ) -> GcResult<Arc<EpochCheckpoint>> {
        if self.entries.read().len() >= self.capacity {
            return Err(GcError::CheckpointOverflow(self.capacity));
        }

        let mut packed: Vec<(u64, i64, Vec<f64>)> = cells
            .iter()
            .map(|(addr, val)| (addr.address.0, addr.tau.0, val.iter().copied().collect()))
            .collect();
        packed.sort_by_key(|(a, t, _)| (*a, *t));

        let bytes_raw: usize = packed.iter().map(|(_, _, v)| v.len() * 8 + 16).sum();
        // Simulated compression ratio: ~0.45 after delta encoding of scalars.
        let bytes_compressed = ((bytes_raw as f64) * 0.45).ceil() as usize;

        let tau_lo = packed.iter().map(|(_, t, _)| *t).min().unwrap_or(0);
        let tau_hi = packed.iter().map(|(_, t, _)| *t).max().unwrap_or(0);

        let mut id_guard = self.next_id.write();
        let id = *id_guard;
        *id_guard += 1;

        let cp = Arc::new(EpochCheckpoint {
            id,
            tau_lo,
            tau_hi,
            cells: packed,
            bytes_raw,
            bytes_compressed,
        });
        self.entries.write().insert(id, Arc::clone(&cp));
        Ok(cp)
    }

    pub fn get(&self, id: u64) -> Option<Arc<EpochCheckpoint>> {
        self.entries.read().get(&id).map(Arc::clone)
    }

    /// Expand a checkpoint back into spacetime cells (for IPC restore).
    pub fn expand(&self, id: u64) -> GcResult<Vec<(SpacetimeAddr, Vec<f64>)>> {
        let cp = self
            .get(id)
            .ok_or_else(|| GcError::Dag(format!("missing checkpoint {id}")))?;
        Ok(cp
            .cells
            .iter()
            .map(|(a, t, v)| (SpacetimeAddr::new(*a, *t), v.clone()))
            .collect())
    }

    pub fn epochs_covered(&self) -> Vec<(Epoch, Epoch)> {
        self.entries
            .read()
            .values()
            .map(|c| (Epoch(c.tau_lo), Epoch(c.tau_hi)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_and_expand_roundtrip_addresses() {
        let store = CheckpointStore::new(8);
        let cells = vec![
            (SpacetimeAddr::new(1, 0), Arc::<[f64]>::from([0.25])),
            (SpacetimeAddr::new(1, 1), Arc::<[f64]>::from([0.5])),
        ];
        let cp = store.compress(cells).unwrap();
        assert!(cp.bytes_compressed < cp.bytes_raw);
        let expanded = store.expand(cp.id).unwrap();
        assert_eq!(expanded.len(), 2);
        assert_eq!(expanded[0].0, SpacetimeAddr::new(1, 0));
    }
}
