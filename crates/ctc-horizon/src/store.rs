use crate::checkpoint::CosmosCheckpoint;
use crate::error::{HorizonError, HorizonResult};
use parking_lot::RwLock;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Persists sealed cosmos checkpoints — in-memory ring plus optional disk archive.
pub struct HorizonStore {
    next_id: AtomicU64,
    capacity: usize,
    entries: RwLock<BTreeMap<u64, CosmosCheckpoint>>,
    disk_root: Option<PathBuf>,
}

impl Default for HorizonStore {
    fn default() -> Self {
        Self::new(64, None)
    }
}

impl HorizonStore {
    pub fn new(capacity: usize, disk_root: Option<PathBuf>) -> Self {
        if let Some(ref root) = disk_root {
            let _ = fs::create_dir_all(root);
        }
        Self {
            next_id: AtomicU64::new(1),
            capacity: capacity.max(1),
            entries: RwLock::new(BTreeMap::new()),
            disk_root,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }

    /// Checkpoint a sealed cosmos beyond the event horizon.
    pub fn checkpoint_cosmos(&self, mut ckpt: CosmosCheckpoint) -> HorizonResult<u64> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        ckpt.id = id;

        {
            let mut entries = self.entries.write();
            if entries.len() >= self.capacity {
                // Evict oldest (lowest id).
                if let Some(oldest) = entries.keys().next().copied() {
                    entries.remove(&oldest);
                }
            }
            if entries.len() >= self.capacity {
                return Err(HorizonError::Capacity(self.capacity));
            }
            entries.insert(id, ckpt.clone());
        }

        if let Some(ref root) = self.disk_root {
            self.write_disk(root, &ckpt)?;
        }
        Ok(id)
    }

    /// Resume the most recent sealed cosmos, if any.
    pub fn resume_latest(&self) -> HorizonResult<Option<CosmosCheckpoint>> {
        if let Some(ckpt) = self.entries.read().values().next_back().cloned() {
            return Ok(Some(ckpt));
        }
        if let Some(ref root) = self.disk_root {
            return self.load_latest_disk(root);
        }
        Ok(None)
    }

    pub fn resume(&self, id: u64) -> HorizonResult<CosmosCheckpoint> {
        if let Some(ckpt) = self.entries.read().get(&id).cloned() {
            return Ok(ckpt);
        }
        if let Some(ref root) = self.disk_root {
            let path = root.join(format!("cosmos-{id}.json"));
            let raw = fs::read_to_string(&path)
                .map_err(|e| HorizonError::Io(format!("{}: {e}", path.display())))?;
            let ckpt: CosmosCheckpoint = serde_json::from_str(&raw)
                .map_err(|e| HorizonError::Corrupt(e.to_string()))?;
            return Ok(ckpt);
        }
        Err(HorizonError::Missing(id))
    }

    fn write_disk(&self, root: &Path, ckpt: &CosmosCheckpoint) -> HorizonResult<()> {
        let path = root.join(format!("cosmos-{}.json", ckpt.id));
        let raw = serde_json::to_string_pretty(ckpt)
            .map_err(|e| HorizonError::Corrupt(e.to_string()))?;
        fs::write(&path, raw).map_err(|e| HorizonError::Io(format!("{}: {e}", path.display())))?;
        Ok(())
    }

    fn load_latest_disk(&self, root: &Path) -> HorizonResult<Option<CosmosCheckpoint>> {
        let mut best: Option<(u64, PathBuf)> = None;
        let rd = match fs::read_dir(root) {
            Ok(rd) => rd,
            Err(_) => return Ok(None),
        };
        for ent in rd.flatten() {
            let name = ent.file_name();
            let name = name.to_string_lossy();
            if let Some(rest) = name.strip_prefix("cosmos-").and_then(|s| s.strip_suffix(".json"))
            {
                if let Ok(id) = rest.parse::<u64>() {
                    if best.as_ref().map(|(b, _)| id > *b).unwrap_or(true) {
                        best = Some((id, ent.path()));
                    }
                }
            }
        }
        let Some((_, path)) = best else {
            return Ok(None);
        };
        let raw = fs::read_to_string(&path)
            .map_err(|e| HorizonError::Io(format!("{}: {e}", path.display())))?;
        let ckpt = serde_json::from_str(&raw).map_err(|e| HorizonError::Corrupt(e.to_string()))?;
        Ok(Some(ckpt))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ctc_entropy::EnergyLedger;
    use ctc_genesis::PhysicalLaws;

    #[test]
    fn checkpoint_and_resume_roundtrip() {
        let store = HorizonStore::new(8, None);
        let id = store
            .checkpoint_cosmos(CosmosCheckpoint {
                id: 0,
                tick: 3,
                laws: PhysicalLaws::default(),
                energy: EnergyLedger::default(),
                boundary: None,
                primary_bulk: vec![0.1, 0.2, 0.3],
                kernel_residual: 1e-12,
                zero_energy: true,
                note: "test".into(),
            })
            .unwrap();
        let resumed = store.resume(id).unwrap();
        assert_eq!(resumed.tick, 3);
        assert_eq!(resumed.primary_bulk, vec![0.1, 0.2, 0.3]);
        assert!(store.resume_latest().unwrap().is_some());
    }
}
